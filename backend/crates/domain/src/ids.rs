//! Strongly-typed id wrappers. Prevents passing a `MessageId` where a `UserId`
//! is expected at compile time — a real bug class in chat code.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
                 Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
            pub fn from_uuid(u: Uuid) -> Self { Self(u) }
            pub fn as_uuid(&self) -> Uuid { self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self { Self(u) }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self { id.0 }
        }
    };
}

id_type!(UserId);
id_type!(WorkspaceId);
id_type!(ChannelId);
id_type!(DmThreadId);
id_type!(MessageId);
