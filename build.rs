use std::path::Path;
use std::{fs::File, io::BufWriter};
use std::io::Write;

use pipl::*;

const PF_PLUG_IN_VERSION: u16 = 13;
const PF_PLUG_IN_SUBVERS: u16 = 28;

#[rustfmt::skip]
fn main() {
    const EFFECT_VERSION_MAJOR: u32 = 0;
    const EFFECT_VERSION_MINOR: u32 = 0;
    const EFFECT_VERSION_PATCH: u32 = 1;

    const EFFECT_NAME: &str = "unmult-rs";

    pipl::plugin_build(vec![
        Property::Kind(PIPLType::AEEffect),
        Property::Name(EFFECT_NAME),
        Property::Category("Sample Plug-ins"),

        #[cfg(target_os = "windows")]
        Property::CodeWin64X86("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacIntel64("EffectMain"),
        #[cfg(target_os = "macos")]
        Property::CodeMacARM64("EffectMain"),

        Property::AE_PiPL_Version { major: 2, minor: 0 },
        Property::AE_Effect_Spec_Version { major: PF_PLUG_IN_VERSION, minor: PF_PLUG_IN_SUBVERS },
        Property::AE_Effect_Version {
            version: EFFECT_VERSION_MAJOR,
            subversion: EFFECT_VERSION_MINOR,
            bugversion: EFFECT_VERSION_PATCH,
            stage: Stage::Develop,
            build: 1,
        },
        Property::AE_Effect_Info_Flags(0),
        Property::AE_Effect_Global_OutFlags(
            OutFlags::PixIndependent |
            OutFlags::NonParamVary |
            OutFlags::DeepColorAware
        ),
        Property::AE_Effect_Global_OutFlags_2(
            OutFlags2::FloatColorAware |
            OutFlags2::SupportsSmartRender |
            OutFlags2::SupportsThreadedRendering |
            OutFlags2::SupportsGetFlattenedSequenceData
        ),
        Property::AE_Effect_Match_Name(EFFECT_NAME),
        Property::AE_Reserved_Info(8),
        Property::AE_Effect_Support_URL("https://www.adobe.com"),
    ]);

    // Generate the LUT
    let out_path = Path::new("src/generated_lut.rs");
    let mut file = BufWriter::new(File::create(out_path).unwrap());

    writeln!(file, "pub static LUT: [u8; 65536] = [").unwrap();

    for i in 0..=0xFFFF {
        let alpha = (i >> 8) as u8;
        let value = (i & 0xFF) as u8;
        let result = if alpha == 0 {
            0
        } else {
            let temp = ((value as u32) << 8) / (alpha as u32);
            if temp > 0xFF {
                0xFF
            } else {
                temp as u8
            }
        };
        writeln!(file, "    {},", result).unwrap();
    }

    writeln!(file, "];").unwrap();
}
