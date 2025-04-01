use std::mem::MaybeUninit;
use std::sync::OnceLock;

use after_effects as ae;
use rgba_to_yuv::{buffers_from_pixels, AbstractRGBAPixel, ArgbPixel, RgbaPixel};

mod rgba_to_yuv;
mod renderer;
static LUT: OnceLock<[u8; 65536]> = OnceLock::new();

// LUT for unpremultiplying alpha
fn get_lut() -> &'static [u8; 65536] {
    LUT.get_or_init(|| {
        let mut lut = [0u8; 65536];
        for (i, lut_value) in lut.iter_mut().enumerate() {
            let alpha = (i >> 8) as u8;
            let value = (i & 0xFF) as u8;

            *lut_value = if alpha == 0 {
                0
            } else {
                let temp = ((value as u32) << 8) / (alpha as u32);
                if temp > 0xFF {
                    0xFF
                } else {
                    temp as u8
                }
            };
        }

        lut[0xFFFF] = 0xFF;
        lut
    })
}

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
}

#[derive(Default)]
struct Plugin { }

ae::define_effect!(Plugin, (), Params);

impl AdobePluginGlobal for Plugin {
    fn can_load(_host_name: &str, _host_version: &str) -> bool {
        true
    }

    fn params_setup(&self, _params: &mut ae::Parameters<Params>, _in_data: InData, _: OutData) -> Result<(), Error> {
        Ok(())
    }

    fn handle_command(&mut self, cmd: ae::Command, in_data: InData, mut out_data: OutData, _params: &mut ae::Parameters<Params>) -> Result<(), ae::Error> {
        match cmd {
            ae::Command::About => {
                self.about(&mut out_data);
            }
            ae::Command::GlobalSetup => {
                self.global_setup(&in_data)?;
            }
            ae::Command::Render { in_layer, out_layer } => {
                self.legacy_render(&in_data, in_layer, out_layer)?;
            }
            ae::Command::SmartPreRender { extra } => {
                self.smart_pre_render(&in_data, extra)?;
            }
            ae::Command::SmartRender { extra } => {
                self.smart_render(extra)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Plugin {
    fn about(&mut self, out_data: &mut OutData) {
        out_data.set_return_msg("SDK_Noise v5.6\rCopyright 2007-2023 Adobe Inc.\rSimple noise effect.");
    }

    fn global_setup(&mut self, in_data: &InData) -> Result<(), ae::Error> {
        win_dbg_logger::DEBUGGER_LOGGER.set_force_log_without_debugger(true);
        log::info!("GlobalSetup");
        // For Premiere - declare supported pixel formats
        if in_data.is_premiere() {
            let suite = ae::pf::suites::PixelFormat::new()?;

            // Add the pixel formats we support in order of preference.
            suite.clear_supported_pixel_formats(in_data.effect_ref())?;
            let formats = [
                ae::pr::PixelFormat::Bgra4444_8u,
                ae::pr::PixelFormat::Bgra4444_16u,
                ae::pr::PixelFormat::Bgra4444_32f,
            ];
            for x in formats {
                suite.add_supported_pixel_format(in_data.effect_ref(), x)?;
            }
        }
        Ok(())
    }

    fn legacy_render(&mut self, in_data: &InData, in_layer: ae::Layer, out_layer: ae::Layer) -> Result<(), ae::Error> {
        if !in_data.is_premiere() {
            // We don't support non-SmartFX unless it's Premiere
            return Err(Error::BadCallbackParameter);
        }

        let in_pixel_format = in_layer.pixel_format()?;
        let out_pixel_format = out_layer.pixel_format()?;

        self.do_render(in_layer, out_layer, in_pixel_format, out_pixel_format)?;
    
        Ok(())
    }

    fn smart_pre_render(&mut self, in_data: &InData, mut extra: ae::PreRenderExtra) -> Result<(), ae::Error> {
        let req = extra.output_request();

        if let Ok(in_result) = extra.callbacks().checkout_layer(0, 0, &req, in_data.current_time(), in_data.time_step(), in_data.time_scale()) {
            let _ = extra.union_result_rect(in_result.result_rect.into());
            let _ = extra.union_max_result_rect(in_result.max_result_rect.into());
        }
        Ok(())
    }

    fn smart_render(&mut self, extra: ae::SmartRenderExtra) -> Result<(), ae::Error> {
        let cb = extra.callbacks();
        let Some(input_world) = cb.checkout_layer_pixels(0)? else {
            return Ok(());
        };

        if let Ok(Some(output_world)) = cb.checkout_output() {
            let in_pixel_format = input_world.pixel_format()?;
            let out_pixel_format = output_world.pixel_format()?;
            self.do_render(input_world, output_world, in_pixel_format, out_pixel_format)?;
        }

        cb.checkin_layer_pixels(0)?;
        Ok(())
    }

    fn do_render(&self, in_layer: ae::Layer, mut out_layer: ae::Layer, in_pixel_format: PixelFormat, out_pixel_format: PixelFormat) -> Result<(), Error> {
        use rayon::prelude::*;

        let in_buffer = in_layer.buffer();
        let out_buffer = out_layer.buffer_mut();
        log::info!("Render {:?} {:?} in_buffer.len() = {}, out_buffer.len() = {}", in_pixel_format, out_pixel_format, in_buffer.len(), out_buffer.len());
        // if in_buffer.len() != out_buffer.len() {
        //     let progress_final = out_layer.height() as _;
        //     in_layer.iterate_with(&mut out_layer, 0, progress_final, None, |_x: i32, _y: i32, pixel: ae::GenericPixel, out_pixel: ae::GenericPixelMut| -> Result<(), Error> {
        //         if _x == 1 && _y == 1 {
        //             match (pixel, out_pixel) {
        //                 (ae::GenericPixel::Pixel8(pixel), ae::GenericPixelMut::Pixel8(out_pixel)) => {
        //                     log::trace!("Pixel8 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //                 }
        //                 (ae::GenericPixel::Pixel16(pixel), ae::GenericPixelMut::Pixel16(out_pixel)) => {
        //                     log::trace!("Pixel16 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //                 }
        //                 (ae::GenericPixel::PixelF32(pixel), ae::GenericPixelMut::PixelF32(out_pixel)) => {
        //                     log::trace!("PixelF32 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //                 }
        //                 _ => {
        //                     log::trace!("BadCallbackParameter");
        //                     return Err(Error::BadCallbackParameter);
        //                 }
        //             }
        //         }
        //         Ok(())
        //     })?;
        //     return Ok(());
        // }

        let in_pixels = ArgbPixel::<u8>::buffer_as_slice_from(in_buffer);
        log::info!("in_pixels.len() = {}", in_pixels.len());
        log::info!("in_pixels[0..40] = {:?}", &in_pixels[0..40]);

        let out_pixels = in_pixels.par_iter().map(|pixel| pixel.unmult_rgba()).collect::<Vec<_>>();
        log::info!("out_pixels.len() = {}", out_pixels.len());
        let out_pixels_buffer = buffers_from_pixels(&out_pixels);
        log::info!("pixels.len() = {}", out_pixels_buffer.len());
        log::info!("pixels[0..40] = {:?}", &out_pixels_buffer[0..40]);

        log::info!("out_buffer.len() = {}", out_buffer.len());
        log::info!("out_buffer[0..40] = {:?}", &out_buffer[0..40]);

        log::info!("out_buffer.len() = {}, out_pixels_buffer.len() = {}, in_buffer.len() = {}", out_buffer.len(), out_pixels_buffer.len(), in_buffer.len());
        // out_buffer.copy_from_slice(pixels);
        out_buffer.copy_from_slice(out_pixels_buffer);
        // out_buffer[0] = 123;
        // out_buffer[4] = 123;
        // out_buffer[8] = 123;
        // out_buffer[out_buffer.len() - 1] = 123;
        log::info!("out_buffer.len() = {}", out_buffer.len());
        log::info!("out_buffer[0..40] = {:?}", &out_buffer[0..40]);
        
        // // buffer.into_par_iter().map(||)
        // let progress_final = out_layer.height() as _;
        // in_layer.iterate_with(&mut out_layer, 0, progress_final, None, |_x: i32, _y: i32, pixel: ae::GenericPixel, out_pixel: ae::GenericPixelMut| -> Result<(), Error> {
        //     if _x == 1 && _y == 1 {
        //         match (pixel, out_pixel) {
        //             (ae::GenericPixel::Pixel8(pixel), ae::GenericPixelMut::Pixel8(out_pixel)) => {
        //                 log::trace!("Pixel8 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //             }
        //             (ae::GenericPixel::Pixel16(pixel), ae::GenericPixelMut::Pixel16(out_pixel)) => {
        //                 log::trace!("Pixel16 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //             }
        //             (ae::GenericPixel::PixelF32(pixel), ae::GenericPixelMut::PixelF32(out_pixel)) => {
        //                 log::trace!("PixelF32 pixel = {:?}, out_pixel = {:?}", pixel, out_pixel);
        //             }
        //             _ => {
        //                 log::trace!("BadCallbackParameter");
        //                 return Err(Error::BadCallbackParameter);
        //             }
        //         }
        //     }
        //     // match (pixel, out_pixel) {
        //     //     (ae::GenericPixel::Pixel8(pixel), ae::GenericPixelMut::Pixel8(out_pixel)) => {
        //     //         let new_pixel = RgbaPixel::new(pixel.red, pixel.green, pixel.blue, pixel.alpha).unmult_rgba();
        //     //         out_pixel.alpha = new_pixel.alpha();
        //     //         out_pixel.red   = new_pixel.red();
        //     //         out_pixel.green = new_pixel.green();
        //     //         out_pixel.blue  = new_pixel.blue();
        //     //     }
        //     //     (ae::GenericPixel::Pixel16(pixel), ae::GenericPixelMut::Pixel16(out_pixel)) => {
        //     //         let new_pixel = RgbaPixel::new(pixel.red, pixel.green, pixel.blue, pixel.alpha).unmult_rgba();
        //     //         out_pixel.alpha = new_pixel.alpha();
        //     //         out_pixel.red   = new_pixel.red();
        //     //         out_pixel.green = new_pixel.green();
        //     //         out_pixel.blue  = new_pixel.blue();
        //     //     }
        //     //     (ae::GenericPixel::PixelF32(pixel), ae::GenericPixelMut::PixelF32(out_pixel)) => {
        //     //         let new_pixel = RgbaPixel::new(pixel.red, pixel.green, pixel.blue, pixel.alpha).unmult_rgba();
        //     //         out_pixel.alpha = new_pixel.alpha();
        //     //         out_pixel.red   = new_pixel.red();
        //     //         out_pixel.green = new_pixel.green();
        //     //         out_pixel.blue  = new_pixel.blue();
        //     //     }
        //     //     _ => return Err(Error::BadCallbackParameter)
        //     // }

        //     Ok(())
        // })?;
        Ok(())
    }
}
