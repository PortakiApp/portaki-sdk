//! In-process test harness for Portaki module renderers and host calls.

mod assertions;
mod fixtures;
mod mock_host;

pub use assertions::SurfaceAssertions;
pub use fixtures::{Booking, GuestIdentityFixture, Property};
pub use mock_host::{MockContext, MockContextBuilder, MockHostFunctions};
