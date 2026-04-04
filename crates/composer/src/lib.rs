use clients::clients::Clients;

use crate::lock::{content::ContentsLock, core::CoresLock, instances::InstancesLock};

pub mod error;
pub mod item;
pub mod lock;
pub mod utils;

/// Предстваляет in-memory хранилище состояния
pub struct State {
    _clients: Clients,
    _instances: InstancesLock,
    _cores: CoresLock,
    _contents: ContentsLock,
}
