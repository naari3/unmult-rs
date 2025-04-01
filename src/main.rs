mod generated_lut;
use generated_lut::LUT;

use after_effects::sys::PF_Pixel;
// static LUT: OnceLock<[u8; 0x10000]> = OnceLock::new();

// // LUT for unpremultiplying alpha
// fn get_lut() -> &'static [u8; 0x10000] {
//     LUT.get_or_init(|| {
//         let mut lut = [0u8; 0x10000];
//         for (i, lut_value) in lut.iter_mut().enumerate() {
//             let alpha = (i >> 8) as u8;
//             let value = (i & 0xFF) as u8;

//             *lut_value = if alpha == 0 {
//                 0
//             } else {
//                 let temp = ((value as u32) << 8) / (alpha as u32);
//                 if temp > 0xFF {
//                     0xFF
//                 } else {
//                     temp as u8
//                 }
//             };
//         }

//         lut[0xFFFF] = 0xFF;
//         lut
//     })
// }
pub fn inner_render_2(pixel: &PF_Pixel, out_pixel: &mut PF_Pixel) {
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

fn main() {
    // win_dbg_logger::DEBUGGER_LOGGER.set_force_log_without_debugger(true);
    // let _ = log::set_logger(&win_dbg_logger::DEBUGGER_LOGGER);
    // log::info!("Hello, world!");

    let input_pixels = [PF_Pixel { red: 0xFF, green: 0, blue: 0, alpha: 0x88 }; 100000000000];
    let mut output_pixels = [PF_Pixel { red: 0, green: 0, blue: 0, alpha: 0 }; 10000000000];
    for (input_pixel, output_pixel) in input_pixels.iter().zip(output_pixels.iter_mut()) {
        inner_render_2(input_pixel, output_pixel);
    }
}
