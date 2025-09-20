pub mod proto;
pub mod proxy_copy_buf;

pub struct ErrFmt<'a>(&'a dyn core::error::Error);

impl core::fmt::Display for ErrFmt<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)?;
        let mut source = self.0.source();
        while let Some(src) = source {
            f.write_fmt(format_args!(" -> {src}"))?;
            source = src.source();
        }
        Ok(())
    }
}

pub fn display_chain(err: &dyn core::error::Error) -> ErrFmt<'_> {
    ErrFmt(err)
}
