use std::collections::HashMap;

use num_enum::FromPrimitive;
use poem_grpc::Request;

/// The type of broker associated with a trading account.
///
/// Parsed from the `broker-type` metadata field as an `i64` value.
/// Falls back to [`BrokerType::Unknown`] for any unrecognized value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(i64)]
pub enum BrokerType {
    /// An unrecognized or missing broker type (default fallback).
    #[num_enum(default)]
    Unknown,
    /// The account belongs to a trader.
    Trader,
    /// The account belongs to a broker.
    Broker,
}

macro_rules! define_values {
    ($($(#[$docs:meta])* ($method:ident, $ty:ty)),*) => {
        $(
            $(#[$docs])*
            #[allow(clippy::wrong_self_convention)]
            fn $method(&self) -> Option<$ty>;
        )*
    };
}

/// Extension trait for extracting common business metadata from a gRPC request.
///
/// All values are read from the request's gRPC metadata (HTTP/2 headers). Methods
/// return `None` when the corresponding header is absent or cannot be parsed into
/// the target type.
///
/// This trait is blanket-implemented for all [`poem_grpc::Request<T>`] types, so it
/// can be used directly on any incoming gRPC request.
///
/// # Examples
///
/// ```rust,ignore
/// use gear_microkit::RequestExt;
///
/// async fn handle(req: poem_grpc::Request<MyMessage>) {
///     if let Some(mid) = req.member_id() {
///         println!("request from member {mid}");
///     }
///
///     let lang = req.accept_language().unwrap_or("en");
///     let features = req.features(); // Vec<&str>
/// }
/// ```
pub trait RequestExt {
    define_values!(
        /// Returns the application identifier from the `app-id` metadata field.
        (app_id, &str),
        /// Returns the client platform (e.g. `"ios"`, `"android"`, `"web"`) from the
        /// `x-platform` metadata field.
        (platform, &str),
        /// Returns the authenticated member ID from the `member-id` metadata field.
        (member_id, u64),
        /// Returns the `Accept-Language` value from the `accept-language` metadata field.
        (accept_language, &str),
        /// Returns the preferred language from the `x-prefer-language` metadata field.
        ///
        /// This takes priority over [`accept_language`](Self::accept_language) when
        /// the client explicitly overrides the display language.
        (prefer_language, &str),
        /// Returns the admin user ID from the `admin-id` metadata field.
        (admin_id, u64),
        /// Returns the cluster identifier from the `x-cluster` metadata field.
        (cluster, &str),
        /// Returns the originating cluster from the `x-from-cluster` metadata field.
        ///
        /// Used for cross-cluster request routing to identify where the request
        /// was originally dispatched from.
        (from_cluster, &str),
        /// Returns the base permission level from the `base-level` metadata field.
        (base_level, i32),
        /// Returns the IP-based geographic region from the `ip-region` metadata field.
        ///
        /// Typically set by the API gateway based on the client's IP address.
        (ip_region, &str),
        /// Returns the user-configured region from the `user-region` metadata field.
        (user_region, &str),
        /// Returns the client user-agent string from the `x-user-agent` metadata field.
        (user_agent, &str),
        /// Returns the client application version (e.g. `"3.2.1"`) from the
        /// `x-application-version` metadata field.
        (application_version, &str),
        /// Returns the client application build number from the `x-application-build`
        /// metadata field.
        (application_build, &str),
        /// Returns the application bundle identifier from the `x-bundle-id` metadata field.
        (bundle_id, &str),
        /// Returns the unique device identifier from the `x-device-id` metadata field.
        (device_id, &str),
        /// Returns the human-readable device name from the `x-device-name` metadata field.
        (device_name, &str),
        /// Returns the device model (e.g. `"iPhone15,2"`) from the `x-device-model`
        /// metadata field.
        (device_model, &str),
        /// Returns the operating member ID from the `op-member-id` metadata field.
        ///
        /// When an admin operates on behalf of a member, this field carries the
        /// admin's identity while [`member_id`](Self::member_id) carries the
        /// target member. Falls back to [`member_id`](Self::member_id) if unset.
        (op_member_id, u64),
        /// Returns the organization ID from the `org-id` metadata field.
        (organization_id, u64),
        /// Returns the target organization ID from the `x-target-org-id` metadata field.
        ///
        /// Used when a request needs to operate in the context of a different
        /// organization than the caller's own.
        (target_organization_id, u64),
        /// Returns the target AAID (Account Aggregation ID) from the `target-aaid`
        /// metadata field.
        (target_aaid, u64),
        /// Returns the user's email address from the `x-email` metadata field.
        (email, &str),
        /// Returns the account channel from the `account-channel` metadata field.
        (account_channel, &str),
        /// Returns the real client IP address from the `x-real-ip` metadata field.
        ///
        /// Typically set by the reverse proxy / API gateway.
        (real_ip, &str)
    );

    /// Returns the per-market permission levels from the `market-levels` metadata field.
    ///
    /// The header value is expected in the format `market1:level1,level2;market2:level3`
    /// where markets are separated by `;` and levels within a market are separated by `,`.
    ///
    /// Returns an empty map if the header is absent or empty.
    fn market_levels(&self) -> HashMap<&str, Vec<&str>>;

    /// Returns the list of feature flags from the `x-features` metadata field.
    ///
    /// Feature flags are expected as a comma-separated string (e.g. `"dark_mode,beta_ui"`).
    /// Returns an empty [`Vec`] if the header is absent.
    fn features(&self) -> Vec<&str>;

    /// Returns the broker type from the `broker-type` metadata field.
    ///
    /// The raw value is parsed as an `i64` and converted via [`BrokerType::from`].
    /// Returns `None` if the header is absent or not a valid integer.
    fn broker_type(&self) -> Option<BrokerType>;
}

macro_rules! impl_string_values {
    ($($(#[$docs:meta])* ($method:ident, $name:literal)),*) => {
        $(
            $(#[$docs])*
            #[inline]
            fn $method(&self) -> Option<&str> {
                self.metadata().get($name)
            }
        )*
    };
}

macro_rules! impl_u64_values {
    ($($(#[$docs:meta])* ($method:ident, $name:literal)),*) => {
        $(
            $(#[$docs])*
            #[inline]
            fn $method(&self) -> Option<u64> {
                self.metadata().get($name).and_then(|value| value.parse().ok())
            }
        )*
    };
}

macro_rules! impl_i32_values {
    ($($(#[$docs:meta])* ($method:ident, $name:literal)),*) => {
        $(
            $(#[$docs])*
            #[inline]
            fn $method(&self) -> Option<i32> {
                self.metadata().get($name).and_then(|value| value.parse().ok())
            }
        )*
    };
}

impl<T> RequestExt for Request<T> {
    impl_string_values!(
        (app_id, "app-id"),
        (platform, "x-platform"),
        (accept_language, "accept-language"),
        (prefer_language, "x-prefer-language"),
        (cluster, "x-cluster"),
        (from_cluster, "x-from-cluster"),
        (ip_region, "ip-region"),
        (user_region, "user-region"),
        (user_agent, "x-user-agent"),
        (application_version, "x-application-version"),
        (application_build, "x-application-build"),
        (bundle_id, "x-bundle-id"),
        (device_id, "x-device-id"),
        (device_name, "x-device-name"),
        (device_model, "x-device-model"),
        (email, "x-email"),
        (account_channel, "account-channel"),
        (real_ip, "x-real-ip")
    );
    impl_u64_values!(
        (member_id, "member-id"),
        (admin_id, "admin-id"),
        (organization_id, "org-id"),
        (target_organization_id, "x-target-org-id"),
        (target_aaid, "target-aaid")
    );
    impl_i32_values!((base_level, "base-level"));

    fn market_levels(&self) -> HashMap<&str, Vec<&str>> {
        let mut levels = HashMap::new();

        if let Some(parts) = self.metadata().get("market-levels") {
            for kv in parts.split(';') {
                if let Some((key, values)) = kv.split_once(';') {
                    levels.insert(key, values.split(',').collect());
                }
            }
        }

        levels
    }

    fn features(&self) -> Vec<&str> {
        self.metadata()
            .get("x-features")
            .map(|value| value.split(',').collect())
            .unwrap_or_default()
    }

    fn op_member_id(&self) -> Option<u64> {
        self.metadata()
            .get("op-member-id")
            .and_then(|value| value.parse::<u64>().ok())
            .or_else(|| self.member_id())
    }

    fn broker_type(&self) -> Option<BrokerType> {
        self.metadata()
            .get("broker-type")
            .and_then(|value| value.parse::<i64>().ok())
            .map(Into::into)
    }
}
