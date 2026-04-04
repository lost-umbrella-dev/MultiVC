use clients::clients::Clients;

use crate::lock::{content::ContentsLock, core::CoresLock, instances::InstancesLock};

pub mod error;
pub mod item;
pub mod lock;
pub mod progress;
pub mod utils;

/// Представляет in-memory хранилище состояния
pub struct State {
    clients: Clients,
    instances: InstancesLock,
    cores: CoresLock,
    contents: ContentsLock,
}

impl State {
    /// Создаёт новый экземпляр состояния
    pub fn new(clients: Clients, instances: InstancesLock, cores: CoresLock, contents: ContentsLock) -> Self {
        Self {
            clients,
            instances,
            cores,
            contents,
        }
    }

    /// Возвращает ссылку на клиенты
    pub fn clients(&self) -> &Clients {
        &self.clients
    }

    /// Возвращает ссылку на lock инстансов
    pub fn instances(&self) -> &InstancesLock {
        &self.instances
    }

    /// Возвращает ссылку на lock ядер
    pub fn cores(&self) -> &CoresLock {
        &self.cores
    }

    /// Возвращает ссылку на lock контент-паков
    pub fn contents(&self) -> &ContentsLock {
        &self.contents
    }
}
