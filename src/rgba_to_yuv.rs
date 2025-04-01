use num_traits::AsPrimitive;

pub trait PixelCompute: Copy + PartialEq + AsPrimitive<f32> + std::fmt::Debug {
    const ZERO: Self;
    const SCALE: f32;
    fn to_f32(self) -> f32;
    fn from_f32(val: f32) -> Self;
}

impl PixelCompute for u8 {
    const ZERO: Self = 0;
    const SCALE: f32 = 255.0;
    fn to_f32(self) -> f32 { (self as f32) / Self::SCALE }
    fn from_f32(val: f32) -> Self { (val * Self::SCALE) as u8 }
}

impl PixelCompute for u16 {
    const ZERO: Self = 0;
    const SCALE: f32 = 65535.0;
    fn to_f32(self) -> f32 { (self as f32) / Self::SCALE }
    fn from_f32(val: f32) -> Self { (val * Self::SCALE) as u16 }
}

impl PixelCompute for f32 {
    const ZERO: Self = 0.0;
    const SCALE: f32 = 1.0;
    fn to_f32(self) -> f32 { self }
    fn from_f32(val: f32) -> Self { val }
}

#[repr(C)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct RgbaPixel<T: PixelCompute> {
    red: T,
    green: T,
    blue: T,
    alpha: T,
}

#[repr(C)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ArgbPixel<T: PixelCompute> {
    alpha: T,
    red: T,
    green: T,
    blue: T,
}

pub trait AbstractRGBAPixel<T: PixelCompute> {
    fn red(&self) -> T;
    fn green(&self) -> T;
    fn blue(&self) -> T;
    fn alpha(&self) -> T;

    fn new(r: T, g: T, b: T, a: T) -> Self;
    fn zero() -> Self;

    fn unmult_rgba(&self) -> Self where Self: Sized {
        let a = self.alpha();
        if a == T::ZERO {
            return Self::zero();
        }
        let r = self.red();
        let g = self.green();
        let b = self.blue();

        let a_f = a.to_f32();
        let mut r_f = r.to_f32();
        let mut g_f = g.to_f32();
        let mut b_f = b.to_f32();

        if a_f < T::SCALE {
            r_f *= a_f;
            g_f *= a_f;
            b_f *= a_f;
        }

        let max_val = max3(r_f, g_f, b_f);
        if max_val > 0.0 {
            let scale = 1.0 / max_val;
            r_f *= scale;
            g_f *= scale;
            b_f *= scale;
        } else {
            return Self::zero();
        }

        Self::new(T::from_f32(r_f), T::from_f32(g_f), T::from_f32(b_f), T::from_f32(max_val))
    }

    
    fn buffer_as_slice_from(data: &[u8]) -> &[Self] where Self: Sized {
        // 長さが4の倍数でなければならない
        log::trace!("data.len() = {}", data.len());
        log::trace!("std::mem::size_of::<Self>() = {}", std::mem::size_of::<Self>());
        assert!(data.len() % std::mem::size_of::<Self>() == 0);
        unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const Self,
                data.len() / std::mem::size_of::<Self>(),
            )
        }
    }
}

// pub trait BufferExt {
//     fn as_buffer(&self) -> &[u8] where Self: Sized {
//         unsafe {
//             std::slice::from_raw_parts(
//                 self as *const Self as *const u8,
//                 std::mem::size_of::<Self>(),
//             )
//         }
//     }
// }


impl<T> AbstractRGBAPixel<T> for RgbaPixel<T> where T: PixelCompute {
    #[inline]
    fn red(&self) -> T { self.red }
    #[inline]
    fn green(&self) -> T { self.green }
    #[inline]
    fn blue(&self) -> T { self.blue }
    #[inline]
    fn alpha(&self) -> T { self.alpha }

    fn new(r: T, g: T, b: T, a: T) -> Self {
        Self { red: r, green: g, blue: b, alpha: a }
    }

    fn zero() -> Self {
        Self { red: T::ZERO, green: T::ZERO, blue: T::ZERO, alpha: T::ZERO }
    }
}

impl<T> AbstractRGBAPixel<T> for ArgbPixel<T> where T: PixelCompute {
    #[inline]
    fn red(&self) -> T { self.red }
    #[inline]
    fn green(&self) -> T { self.green }
    #[inline]
    fn blue(&self) -> T { self.blue }
    #[inline]
    fn alpha(&self) -> T { self.alpha }

    fn new(r: T, g: T, b: T, a: T) -> Self {
        Self { red: r, green: g, blue: b, alpha: a }
    }

    fn zero() -> Self {
        Self { red: T::ZERO, green: T::ZERO, blue: T::ZERO, alpha: T::ZERO }
    }
}

// #[derive(Debug, Default, PartialEq, Clone)]
// pub struct YuvaPixel<T: PixelCompute> {
//     y: T,
//     u: T,
//     v: T,
//     alpha: T,
// }

// impl<T> YuvaPixel<T> where T: PixelCompute {
//     pub fn new(y: T, u: T, v: T, a: T) -> Self {
//         Self { y, u, v, alpha: a }
//     }

//     #[inline]
//     pub fn get_y(&self) -> T { self.y }
//     #[inline]
//     pub fn get_u(&self) -> T { self.u }
//     #[inline]
//     pub fn get_v(&self) -> T { self.v }
//     #[inline]
//     pub fn get_alpha(&self) -> T { self.alpha }

//     pub fn zero() -> Self {
//         Self { y: T::ZERO, u: T::ZERO, v: T::ZERO, alpha: T::ZERO }
//     }
// }

// impl<T> From<RgbaPixel<T>> for YuvaPixel<T> where T: PixelCompute {
//     fn from(rgba: RgbaPixel<T>) -> Self {
//         let r = rgba.red.to_f32();
//         let g = rgba.green.to_f32();
//         let b = rgba.blue.to_f32();
//         let a = rgba.alpha.to_f32();

//         let y = 0.299 * r + 0.587 * g + 0.114 * b;
//         let u = -0.168_935  * r - 0.331_665  * g + 0.500_59 * b;
//         let v = 0.499_813  * r - 0.418_531  * g - 0.081_282 * b;

//         YuvaPixel::<T>::new(T::from_f32(y), T::from_f32(u), T::from_f32(v), T::from_f32(a))
//     }
// }

// impl<T> From<YuvaPixel<T>> for RgbaPixel<T>
// where
//     T: PixelCompute,
// {
//     fn from(yuva: YuvaPixel<T>) -> Self {
//         let y = yuva.y.to_f32();
//         let u = yuva.u.to_f32();
//         let v = yuva.v.to_f32();
//         let a = yuva.alpha.to_f32();

//         // 逆変換行列（概ね）の係数
//         let r = y - 0.0 + 1.403 * v;
//         let g = y - 0.344 * u - 0.714 * v;
//         let b = y + 1.770 * u + 0.0;

//         RgbaPixel::<T>::new(
//             T::from_f32(r),
//             T::from_f32(g),
//             T::from_f32(b),
//             T::from_f32(a),
//         )
//     }
// }

fn max3<T: PartialOrd>(a: T, b: T, c: T) -> T {
    if a >= b && a >= c { a } else if b >= c { b } else { c }
}

pub fn buffers_from_pixels<T: PixelCompute, U: AbstractRGBAPixel<T>>(pixels: &[U]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            pixels.as_ptr() as *const u8,
            std::mem::size_of_val(pixels),
        )
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_rgba_pixel() {
        let p = RgbaPixel::<u8>::new(1, 2, 3, 4);
        assert_eq!(p.red(), 1);
        assert_eq!(p.green(), 2);
        assert_eq!(p.blue(), 3);
        assert_eq!(p.alpha(), 4);
    }

    // #[test]
    // fn test_yuva_pixel() {
    //     let p = YuvaPixel::<u8>::new(1, 2, 3, 4);
    //     assert_eq!(p.get_y(), 1);
    //     assert_eq!(p.get_u(), 2);
    //     assert_eq!(p.get_v(), 3);
    //     assert_eq!(p.get_alpha(), 4);
    // }

    #[test]
    fn test_pixel_compute() {
        assert_eq!(u8::ZERO, 0);
        assert_eq!(u8::SCALE, 255.0);
        assert_eq!(u8::from_f32(0.5), 127);
        assert_eq!(u8::from_f32(0.0), 0);
        assert_eq!(u8::from_f32(1.0), 255);
        assert_eq!(u8::from_f32(0.1), 25);
        assert_eq!(u8::from_f32(0.9), 229);
        assert_eq!(u8::from_f32(0.99), 252);
        assert_eq!(u8::from_f32(0.01), 2);
        assert_eq!(u8::from_f32(0.001), 0);
        assert_eq!(u8::from_f32(0.999), 254);

        assert_eq!(u16::ZERO, 0);
        assert_eq!(u16::SCALE, 65535.0);
        assert_eq!(u16::from_f32(0.5), 32767);
        assert_eq!(u16::from_f32(0.0), 0);
        assert_eq!(u16::from_f32(1.0), 65535);
        assert_eq!(u16::from_f32(0.1), 6553);
        assert_eq!(u16::from_f32(0.9), 58981);
        assert_eq!(u16::from_f32(0.99), 64879);
        assert_eq!(u16::from_f32(0.01), 655);
        assert_eq!(u16::from_f32(0.001), 65);
        assert_eq!(u16::from_f32(0.999), 65469);

        assert_eq!(f32::ZERO, 0.0);
        assert_eq!(f32::SCALE, 1.0);
        assert_eq!(f32::from_f32(0.5), 0.5);
        assert_eq!(f32::from_f32(0.0), 0.0);
        assert_eq!(f32::from_f32(1.0), 1.0);
        assert_eq!(f32::from_f32(0.1), 0.1);   
    }

    #[test]
    fn test_rgba_pixel_zero() {
        let p = RgbaPixel::<u8>::zero();
        assert_eq!(p.red(), 0);
        assert_eq!(p.green(), 0);
        assert_eq!(p.blue(), 0);
        assert_eq!(p.alpha(), 0);
    }

    // #[test]
    // fn test_yuva_pixel_zero() {
    //     let p = YuvaPixel::<u8>::zero();
    //     assert_eq!(p.get_y(), 0);
    //     assert_eq!(p.get_u(), 0);
    //     assert_eq!(p.get_v(), 0);
    //     assert_eq!(p.get_alpha(), 0);
    // }

    #[test]
    fn test_rgba_pixel_from_f32() {
        let p = RgbaPixel::<u8>::new(1, 2, 3, 4);
        let p_f32 = RgbaPixel::<f32>::new(p.red.to_f32(), p.green.to_f32(), p.blue.to_f32(), p.alpha.to_f32());
        assert_eq!(p_f32.red, 1.0 / 255.0);
        assert_eq!(p_f32.green, 2.0 / 255.0);
        assert_eq!(p_f32.blue, 3.0 / 255.0);
        assert_eq!(p_f32.alpha, 4.0 / 255.0);
    }

    // #[test]
    // fn test_yuva_pixel_from_f32() {
    //     let p = YuvaPixel::<u8>::new(1, 2, 3, 4);
    //     let p_f32 = YuvaPixel::<f32>::new(p.y.to_f32(), p.u.to_f32(), p.v.to_f32(), p.alpha.to_f32());
    //     assert_eq!(p_f32.y, 1.0 / 255.0);
    //     assert_eq!(p_f32.u, 2.0 / 255.0);
    //     assert_eq!(p_f32.v, 3.0 / 255.0);
    //     assert_eq!(p_f32.alpha, 4.0 / 255.0);
    // }

    #[test]
    fn test_rgba_pixel_to_f32() {
        let p = RgbaPixel::<u8>::new(1, 2, 3, 4);
        assert_eq!(p.red.to_f32(), 1.0 / 255.0);
        assert_eq!(p.green.to_f32(), 2.0 / 255.0);
        assert_eq!(p.blue.to_f32(), 3.0 / 255.0);
        assert_eq!(p.alpha.to_f32(), 4.0 / 255.0);
    }

    // #[test]
    // fn test_yuva_pixel_to_f32() {
    //     let p = YuvaPixel::<u8>::new(1, 2, 3, 4);
    //     assert_eq!(p.y.to_f32(), 1.0 / 255.0);
    //     assert_eq!(p.u.to_f32(), 2.0 / 255.0);
    //     assert_eq!(p.v.to_f32(), 3.0 / 255.0);
    //     assert_eq!(p.alpha.to_f32(), 4.0 / 255.0);
    // }

    // #[test]
    // fn test_rgba_to_yuva() {
    //     let rgba = RgbaPixel::<f32>::new(0.0, 0.0, 1.0, 1.0);
    //     let yuva = YuvaPixel::<f32>::from(rgba.clone());
    //     let converted_rgba = RgbaPixel::<f32>::from(yuva);
    //     assert!((rgba.red - converted_rgba.red).abs() < 1e-3);
    //     assert!((rgba.green - converted_rgba.green).abs() < 1e-3);
    //     assert!((rgba.blue - converted_rgba.blue).abs() < 1e-3);
    //     assert!((rgba.alpha - converted_rgba.alpha).abs() < 1e-3);
    // }

    // #[test]
    // fn test_yuva_to_rgba() {
    //     let rgba = RgbaPixel::<f32>::new(0.392_156_87, 0.588_235_3, 0.784_313_74, 1.0);
    //     let yuva = YuvaPixel::<f32>::from(rgba);
    //     let converted_rgba = RgbaPixel::<f32>::from(yuva.clone());
    //     let converted_yuva = YuvaPixel::<f32>::from(converted_rgba);
    //     assert!((yuva.y - converted_yuva.y).abs() < 1e-3);
    //     assert!((yuva.u - converted_yuva.u).abs() < 1e-3);
    //     assert!((yuva.v - converted_yuva.v).abs() < 1e-3);
    //     assert!((yuva.alpha - converted_yuva.alpha).abs() < 1e-3);
    // }

    fn unmult_rgba8(r: u8, g: u8, b: u8, a: u8) -> Option<(u8, u8, u8, u8)> {
        if a == 0 {
            Some((0, 0, 0, 0))
        } else {
            let a = a as f32;
            let mut r = r as f32;
            let mut g = g as f32;
            let mut b = b as f32;
            if a < 255.0 {
                r = (r * a) / 255.0;
                g = (g * a) / 255.0;
                b = (b * a) / 255.0;
            }
    
            let max_val = max3(r, g, b) as f32;
            if max_val > 0.0 {
                let scale = 255.0 / max_val;
                r *= scale;
                g *= scale;
                b *= scale;
            } else {
                return Some((0, 0, 0, 0));
            }
            Some((r as u8, g as u8, b as u8, max_val as u8))
        }
    }
    
    #[test]
    fn test_unmult_rgba8() {
        let (r, g, b, a) = unmult_rgba8(0, 0, 0, 255).unwrap();
        assert_eq!((r, g, b, a), (0, 0, 0, 0));

        let (r, g, b, a) = unmult_rgba8(255, 255, 255, 255).unwrap();
        assert_eq!((r, g, b, a), (255, 255, 255, 255));

        let (r, g, b, a) = unmult_rgba8(255, 255, 255, 128).unwrap();
        assert_eq!((r, g, b, a), (255, 255, 255, 128));

        let (r, g, b, a) = unmult_rgba8(255, 255, 255, 0).unwrap();
        assert_eq!((r, g, b, a), (0, 0, 0, 0));

        let (r, g, b, a) = unmult_rgba8(255, 127, 127, 255).unwrap();
        assert_eq!((r, g, b, a), (255, 127, 127, 255));
    }

    #[test]
    fn test_pixel_unmult_u8() {
        let p = RgbaPixel::<u8>::new(0, 0, 0, 255).unmult_rgba();
        assert_eq!(p, RgbaPixel::<u8>::zero());

        let p = RgbaPixel::<u8>::new(255, 255, 255, 255).unmult_rgba();
        assert_eq!(p, RgbaPixel::<u8>::new(255, 255, 255, 255));

        let p = RgbaPixel::<u8>::new(255, 255, 255, 128).unmult_rgba();
        assert_eq!(p, RgbaPixel::<u8>::new(255, 255, 255, 128));

        let p = RgbaPixel::<u8>::new(255, 255, 255, 0).unmult_rgba();
        assert_eq!(p, RgbaPixel::<u8>::zero());

        let p = RgbaPixel::<u8>::new(255, 127, 127, 255).unmult_rgba();
        assert_eq!(p, RgbaPixel::<u8>::new(255, 127, 127, 255));
    }

    #[test]
    fn test_pixels_buffer_as_slice_from_rgba() {
        let slice = [0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255];
        let pixels = RgbaPixel::<u8>::buffer_as_slice_from(&slice);
        assert_eq!(pixels.len(), 5);
        assert_eq!(pixels[0], RgbaPixel::<u8>::new(0, 0, 0, 255));
        let buffer = buffers_from_pixels(pixels);
        assert_eq!(slice, buffer);
    }

    #[test]
    fn test_pixels_buffer_as_slice_from_argb() {
        let slice = [255, 0, 0, 0, 255, 127, 127, 127, 255, 255, 255, 255, 255, 0, 0, 0];
        let pixels = ArgbPixel::<u8>::buffer_as_slice_from(&slice);
        assert_eq!(pixels.len(), 4);
        assert_eq!(pixels[0], ArgbPixel::<u8>::new(0, 0, 0, 255));
        let buffer = buffers_from_pixels(pixels);
        assert_eq!(slice, buffer);
    }
}
