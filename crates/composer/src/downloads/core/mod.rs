//! Установка ядер.
//!
//! Разделён на логические части:
//! - [`executable`] — определение и переименование исполняемого файла ядра под текущую ОС.
//! - [`pipeline`] — полный pipeline: скачивание → распаковка → переименование → хэширование → commit.
//! - [`install`] — точка входа: [`State::install_cores`](crate::State::install_cores).
pub mod executable;
pub mod install;
pub mod pipeline;
