use std::path::Path;

pub(crate) fn apply(prefix: &Path, dpi: u32) -> Result<(), String> {
    super::userreg::set(
        prefix,
        &[(
            "Control Panel\\Desktop",
            vec![("LogPixels", format!("dword:{dpi:08x}"))],
        )],
    )
}
