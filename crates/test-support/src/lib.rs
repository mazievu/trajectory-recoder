//! Testing utilities, mock event generators, synthetic UIA trees, and named pipe fixtures.

pub mod fake_inputs;
pub mod generators;
pub mod mock_events;
pub mod mock_ipc;
pub mod mock_spool;
pub mod synthetic_uia;

pub use fake_inputs::FakeInputDriver;
pub use mock_events::MockEventGenerator;
pub use mock_ipc::MockNamedPipePair;
pub use mock_spool::MockSpoolFixture;
pub use synthetic_uia::{SyntheticUiaElement, SyntheticUiaTree};
