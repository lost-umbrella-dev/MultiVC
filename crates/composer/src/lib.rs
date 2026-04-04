use clients::clients::Clients;

use crate::lock::{content::ContentsLock, core::CoresLock, instances::InstancesLock};

pub mod error;
pub mod item;
pub mod lock;
pub mod utils;

/// Предстваляет in-memory хранилище состояния
pub struct State {
    clients: Clients,
    instances: InstancesLock,
    cores: CoresLock,
    contents: ContentsLock,
}
