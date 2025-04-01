
#![feature(test)]
extern crate test;

mod generated_lut;
use generated_lut::LUT;

use after_effects::{self as ae, sys::PF_Pixel};
use rgba_to_yuv::RgbaPixel;

mod rgba_to_yuv;

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

        self.do_render(in_layer, out_layer)?;
    
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
            self.do_render(input_world, output_world)?;
        }

        cb.checkin_layer_pixels(0)?;
        Ok(())
    }

    fn do_render(&self, in_layer: ae::Layer, mut out_layer: ae::Layer) -> Result<(), Error> {
        let progress_final = out_layer.height() as _;
        in_layer.iterate_with(&mut out_layer, 0, progress_final, None, |_x: i32, _y: i32, pixel: ae::GenericPixel, out_pixel: ae::GenericPixelMut| -> Result<(), Error> {
            match (pixel, out_pixel) {
                (ae::GenericPixel::Pixel8(pixel), ae::GenericPixelMut::Pixel8(out_pixel)) => {
                    inner_render(pixel, out_pixel);
                }
                (ae::GenericPixel::Pixel16(pixel), ae::GenericPixelMut::Pixel16(out_pixel)) => {
                    let new_pixel = RgbaPixel::new(pixel.red, pixel.green, pixel.blue, pixel.alpha).unmult_rgba();
                    out_pixel.alpha = new_pixel.get_alpha();
                    out_pixel.red   = new_pixel.get_red();
                    out_pixel.green = new_pixel.get_green();
                    out_pixel.blue  = new_pixel.get_blue();
                }
                (ae::GenericPixel::PixelF32(pixel), ae::GenericPixelMut::PixelF32(out_pixel)) => {
                    let new_pixel = RgbaPixel::new(pixel.red, pixel.green, pixel.blue, pixel.alpha).unmult_rgba();
                    out_pixel.alpha = new_pixel.get_alpha();
                    out_pixel.red   = new_pixel.get_red();
                    out_pixel.green = new_pixel.get_green();
                    out_pixel.blue  = new_pixel.get_blue();
                }
                _ => return Err(Error::BadCallbackParameter)
            }

            Ok(())
        })?;
        Ok(())
    }
}

pub fn inner_render(pixel: &PF_Pixel, out_pixel: &mut PF_Pixel) {
    let a = pixel.alpha;
    let r = pixel.red;
    let g = pixel.green;
    let b = pixel.blue;

    let max_rgb = r.max(g).max(b);
    let offset = (max_rgb as usize) << 8;

    #[allow(arithmetic_overflow)]
    let a = (a * max_rgb) >> 8;
    out_pixel.alpha = a;
    out_pixel.red   = LUT[offset + r as usize];
    out_pixel.green = LUT[offset + g as usize];
    out_pixel.blue  = LUT[offset + b as usize];
}

pub fn inner_render_2(pixel: &PF_Pixel, out_pixel: &mut PF_Pixel) {
    inner_render(pixel, out_pixel);
    // let a = pixel.alpha;
    // let r = pixel.red;
    // let g = pixel.green;
    // let b = pixel.blue;

    // let max_rgb = r.max(g).max(b);
    // let offset = (max_rgb as usize) << 8;

    // #[allow(arithmetic_overflow)]
    // let a = (a * max_rgb) >> 8;
    // // out_pixel.alpha = a;
    // // out_pixel.red   = LUT2[offset + r as usize];
    // // out_pixel.green = LUT2[offset + g as usize];
    // // out_pixel.blue  = LUT2[offset + b as usize];
    // let packed: u32 = (a as u32) | ((LUT2[offset + r as usize] as u32) << 8) | ((LUT2[offset + g as usize] as u32) << 16) | ((LUT2[offset + b as usize] as u32) << 24);

    // unsafe {
    //     *(out_pixel as *mut PF_Pixel as *mut u32) = packed;
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_inner_render() {
        let input_pixel = PF_Pixel { red: 0xFF, green: 0, blue: 0, alpha: 0x88 };
        let mut output_pixel = PF_Pixel { red: 0, green: 0, blue: 0, alpha: 0 };
        inner_render(&input_pixel, &mut output_pixel);
        assert_eq!(output_pixel.red, 0xFF);
        assert_eq!(output_pixel.green, 0);
        assert_eq!(output_pixel.blue, 0);
        assert_eq!(output_pixel.alpha, 0x88);
    }

    #[test]
    fn test_inner_render_2() {
        let input_pixel = PF_Pixel { red: 0xFF, green: 0, blue: 0, alpha: 0x88 };
        let mut output_pixel = PF_Pixel { red: 0, green: 0, blue: 0, alpha: 0 };
        inner_render_2(&input_pixel, &mut output_pixel);
        assert_eq!(output_pixel.red, 0xFF);
        assert_eq!(output_pixel.green, 0);
        assert_eq!(output_pixel.blue, 0);
        assert_eq!(output_pixel.alpha, 0x88);
    }

    #[bench]
    fn bench_inner_render(b: &mut Bencher) {
        let input_pixels = vec![PF_Pixel { red: 0xFF, green: 0, blue: 0, alpha: 0x88 }; 100000];
        let mut output_pixels = vec![PF_Pixel { red: 0, green: 0, blue: 0, alpha: 0 }; 100000];
        b.iter(|| {
            for (input_pixel, output_pixel) in input_pixels.iter().zip(output_pixels.iter_mut()) {
                inner_render(input_pixel, output_pixel);
            }
        });
    }

    #[bench]
    fn bench_inner_render_2(b: &mut Bencher) {
        let input_pixels = vec![PF_Pixel { red: 0xFF, green: 0, blue: 0, alpha: 0x88 }; 100000];
        let mut output_pixels = vec![PF_Pixel { red: 0, green: 0, blue: 0, alpha: 0 }; 100000];
        b.iter(|| {
            for (input_pixel, output_pixel) in input_pixels.iter().zip(output_pixels.iter_mut()) {
                inner_render_2(input_pixel, output_pixel);
            }
        });
    }
}
