mod cd;
mod echo;
mod executable;
mod pwd;
mod r#type;

pub(crate) use cd::handle_cd;
pub(crate) use echo::handle_echo;
pub(crate) use executable::handle_executable;
pub(crate) use pwd::handle_pwd;
pub(crate) use r#type::handle_type;
