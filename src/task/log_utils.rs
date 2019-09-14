use std::fmt::Arguments;

/// This struct only exists because kv logging isn't supported from the macros right now.
pub(crate) struct LogData {
    pub parent_id: u64,
    pub child_id: u64,
}

impl<'a> log::kv::Source for LogData {
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::Visitor<'kvs>,
    ) -> Result<(), log::kv::Error> {
        visitor.visit_pair("parent_id".into(), self.parent_id.into())?;
        visitor.visit_pair("child_id".into(), self.child_id.into())?;
        Ok(())
    }
}

pub fn print(msg: Arguments<'_>, key_values: impl log::kv::Source) {
    log::logger().log(
        &log::Record::builder()
            .args(msg)
            .key_values(&key_values)
            .level(log::Level::Trace)
            .target(module_path!())
            .module_path(Some(module_path!()))
            .file(Some(file!()))
            .line(Some(line!()))
            .build(),
    );
}
