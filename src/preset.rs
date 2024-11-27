#[derive(clap::ValueEnum, Clone)]
pub enum PresetBarcodeFormats {
    /// @BARCODE_UMI format as produced by Flexiplex for 10x3 chemistry
    BcUmi,

    /// `_<UMI>` format as produced by `umi-tools extract`.
    UmiTools,

    /// bcl2fastq format, which has `:<UMI>` at the end of the read ID.
    Illumina,
}

pub fn get_barcode_regex(preset: &PresetBarcodeFormats) -> String {
    match preset {
        PresetBarcodeFormats::BcUmi => { String::from(r"^([ATCG]{16})_([ATCG]{12})") }
        PresetBarcodeFormats::UmiTools => { String::from(r"_([ATCG]+)$") }
        PresetBarcodeFormats::Illumina => { String::from(r":([ATCG]+)$") }
    }
}