use async_trait::async_trait;
use thiserror::Error;

/// Used for establishing connections to a database. Implementations can be found in the `hextacy_derive`
/// crate. Manual implementations should utilise `hextacy::drivers`.
#[async_trait]
pub trait RepositoryAccess<C> {
    async fn connect(&self) -> Result<C, DatabaseError>;
}

/// Used for creating bounds on generic connections when the adapter needs to have atomic repository access.
///
/// This trait is used to normalise the API for transactions that are connection based and transactions that
/// return a transaction struct.
///
/// When transactions are connection based, the `TransactionResult` is typically
/// the connection on which the transaction is started.
///
/// When they are struct based, the adapter must implement a repository trait for both the
/// connection and transaction.
///
/// Check out the [driver module][crate::drivers::db] to see concrete implementations.
#[async_trait]
pub trait Atomic: Sized {
    type TransactionResult: Send;
    async fn start_transaction(self) -> Result<Self::TransactionResult, DatabaseError>;
    async fn commit_transaction(tx: Self::TransactionResult) -> Result<(), DatabaseError>;
    async fn abort_transaction(tx: Self::TransactionResult) -> Result<(), DatabaseError>;
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Error while attempting to establish connection: {0}")]
    Driver(#[from] super::drivers::DriverError),

    #[cfg(any(feature = "db", feature = "full", feature = "diesel"))]
    #[error("Diesel Error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[cfg(any(feature = "db", feature = "full", feature = "mongo"))]
    #[error("Mongo Error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[cfg(any(feature = "db", feature = "full", feature = "seaorm"))]
    #[error("SeaORM Error: {0}")]
    SeaORM(#[from] sea_orm::DbErr),
}

#[macro_export]
/// Generates a `Repository` struct (or a custom name) with `pub(super)` visibility and derives [RepositoryAccess].
///
/// Useful for reducing overall boilerplate in repository adapters.
///
/// #### 1 - Struct ident (optional)
///
/// The macro accepts an optional ident as the first parameter and will name the struct that way if provided.
///
/// #### 2 - Driver - connection, field - driver pairs
///
/// The second part of the macro uses a
///
/// `DriverIdent => ConnectionIdent,`
///
/// `field_ident => driver,`
///
/// syntax, where DriverIdent and ConnectionIdent are arbitrary driver and connection generics that can be used
/// to specify which repositories will use which drivers. Available drivers (for `field_ident`) are `diesel`, `seaorm` for postgres
/// and `mongo`.
///
/// #### 3 - Repository ident - Repository path
///
/// The third and final part accepts a
///
/// `RepoIdent => SomeRepository<ConnectionIdent>`
///
/// syntax, indicating which identifiers can call which repository methods.
///
/// The drivers module includes drivers which derive DBConnect for the derived
/// connections. Check out [Repository][hextacy_derive::Repository]
///
/// ### Example
///
/// ```ignore
/// adapt! {
///     Adapter, // Optional name for the generated struct    
///
///     Postgres => PgConnection, // Driver and connection
///     postgres => diesel,       // The struct field to annotate with a driver.
///
///     Mongo    => MgConnection, // Same as above, any number of pairs
///     mongo    => mongo;        // is allowed
///
///     SomeRepo => SomeRepository<Conn>, // Repository bounds
///     OtherRepo => OtherRepository<Conn>,
///     /* ... */
/// }
/// ```
///
/// This macro also provides a `new()` method whose input is anything that implements `DBConnect` for convenience.
/// `DBConnect` is automatically added to the bounds as a generic parameter for the driver.
macro_rules! adapt {
    (
        $(
            $driver:ident => $conn_name:ident,
            $field:ident  => $driver_field:ident $(,)?
        )+;
        $(
            $id:ident     => $repo_bound:path
        ),*
        ) => {
               #[allow(non_snake_case)]
               #[derive(Debug, Clone, hextacy::derive::Repository)]
               pub struct Repository<$($driver),+, $($conn_name),+, $($id),*>
               where
                  $(
                      $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name>,
                  )+
                   $($id: $repo_bound),*
               {
                  $(
                      #[$driver_field($conn_name)]
                      $field: hextacy::drivers::db::Driver<$driver, $conn_name>,
                  )+
                   $($id: ::std::marker::PhantomData<$id>),*
               }

               #[allow(non_snake_case)]
               impl<$($driver),+, $($conn_name),+, $($id),*> Repository<$($driver),+, $($conn_name),+, $($id),*>
               where
                  $(
                      $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name>,
                  )+
                   $($id: $repo_bound),*
               {
                   pub fn new($($driver: ::std::sync::Arc<$driver>),+) -> Self {
                       Self {
                          $(
                              $field: hextacy::drivers::db::Driver::new($driver),
                          )+
                           $($id: ::std::marker::PhantomData),*
                       }
                   }
               }
          };
    (
        $custom_name:ident,
        $(
            $driver:ident => $conn_name:ident,
            $field:ident  => $driver_field:ident $(,)?
        )+;
        $(
            $id:ident     => $repo_bound:path
        ),*
        ) => {
               #[allow(non_snake_case)]
               #[derive(Debug, Clone, hextacy::derive::Repository)]
               pub struct $custom_name<$($driver),+, $($conn_name),+, $($id),*>
               where
                  $(
                      $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name> + Send + Sync,
                  )+
                   $($id: $repo_bound + Send + Sync),*
               {
                  $(
                      #[$driver_field($conn_name)]
                      $field: hextacy::drivers::db::Driver<$driver, $conn_name>,
                  )+
                   $($id: ::std::marker::PhantomData<$id>),*
               }

               #[allow(non_snake_case)]
               impl<$($driver),+, $($conn_name),+, $($id),*> $custom_name<$($driver),+, $($conn_name),+, $($id),*>
               where
                  $(
                      $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name> + Send + Sync,
                  )+
                   $($id: $repo_bound + Send + Sync),*
               {
                   pub fn new($($driver: ::std::sync::Arc<$driver>),+) -> Self {
                       Self {
                          $(
                              $field: hextacy::drivers::db::Driver::new($driver),
                          )+
                           $($id: ::std::marker::PhantomData),*
                       }
                   }
               }
          };
}

#[macro_export]
/// Used to implement an api for any adapter used by business level services and reducing boilerplate
/// associated with adapter generics.
///
/// The following syntax, similar to the [adapt] macro, is accepted:
///
/// ```ignore
/// api_impl! {
///     // Implements API for Implementor
///     Implementor => API;
///
///     // Specifies which drivers will use which type of connections
///     Driver => Connection ();
///
///     // Naming the bounds through which the repository methods can be called
///     // and the connections they will use
///     User => UserRepository : Connection,
///     /* ... */
/// }
///
/// ```
/// The first `ident => path` pair specifies the api to implement (right) and the struct on which
/// to implement it (left).
///
/// The second pair of parameters are any number of `ident => path` pairs representing how the repositories will be named in the impl block.
/// From the example above, a `U` generic will be created in place of a `UserRepository`, therefore accessing its methods
/// is done via `U::method(/* .. */)`.
///
/// The last pair of parameters are any number of function items for the trait implementation.
///
/// The first three pairs of arguments are used for the bounds in the api implementation, while the fourth (the function items)
/// are used to generate the impl block.
///
/// This macro is mostly for utility and hiding the wall of bounds required for service adapters and for standard
/// cases where you just need repositories with and without atomic connections.
macro_rules! api_impl {
    // This one's for connections that need to have Atomic
    (
     // Implementing struct : API to implement
     $struct:ident : $api:path,

     // Driver => Connection generic, connection bounds
     $($driver:ident => $conn_name:ident : $($(+)? $conn_trait:path)+),+;

     // Generic param => Repository bound : connection bound
     $($id:ident => $bound:ident : $conn_bound:ident),*;

     $($b:item)*
    ) => {
        #[async_trait::async_trait]
        impl
            <$($driver),+, $($conn_name),+, $($id),*> $api
        for
            $struct<$($driver),+, $($conn_name),+, $($id),*>
        where
            Self: $(hextacy::db::RepositoryAccess<$conn_name> +)+,

            // Apply DBConnect bounds for drivers
            $(
                $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name> + Send + Sync,
            )+

            // Apply connection bounds
            $(
                $conn_name: $($conn_trait)+ + Send
            )+,

            // Apply repository bounds
            $($id: $bound<$conn_bound> + $bound<<$conn_bound as hextacy::db::Atomic>::TransactionResult> + Send + Sync),*

            {
                $($b)*
            }
    };

    // For repositories without atomic connections.
    (
     // Implementing struct : API to implement
     $struct:ident : $api:path,

     // Driver => Connection generic, connection bounds
     $($driver:ident => $conn_name:ident),+;

     // Generic param => Repository bound : connection bound
     $($id:ident => $bound:ident : $conn_bound:ident),*;

     $($b:item)*
    ) => {
        #[async_trait::async_trait]
        impl
            <$($driver),+, $($conn_name),+, $($id),*> $api
        for
            $struct<$($driver),+, $($conn_name),+, $($id),*>
        where
            Self: $(hextacy::db::RepositoryAccess<$conn_name> +)+,

            // Apply DBConnect bounds for drivers
            $(
                $driver: hextacy::drivers::db::DBConnect<Connection = $conn_name>,
            )+

            // Apply repository bounds
            $($id: $bound<$conn_bound>),*

            {
                $($b)*
            }
    };
}
