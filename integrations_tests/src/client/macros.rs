#[macro_export]
macro_rules! generate_pocket_query_call {
    ($method_name:ident) => {
        #[allow(dead_code)]
        pub fn $method_name(
            pic: &pocket_ic::PocketIc,
            sender: candid::Principal,
            canister_id: candid::Principal,
            args: &$method_name::Args,
        ) -> $method_name::Response {
            let method_name = stringify!($method_name);

            $crate::client::pocket::execute_query(pic, sender, canister_id, method_name, args)
        }
    };
}

#[macro_export]
macro_rules! generate_pocket_update_call {
    ($method_name:ident) => {
        #[allow(dead_code)]
        pub fn $method_name(
            pic: &mut pocket_ic::PocketIc,
            sender: candid::Principal,
            canister_id: candid::Principal,
            args: &$method_name::Args,
        ) -> $method_name::Response {
            let method_name = stringify!($method_name);

            $crate::client::pocket::execute_update(pic, sender, canister_id, method_name, args)
        }
    };
}

#[macro_export]
macro_rules! generate_update_call_encoded_args {
    ($method_name:ident) => {
        #[allow(dead_code)]
        pub fn $method_name(
            pic: &mut pocket_ic::PocketIc,
            sender: candid::Principal,
            canister_id: candid::Principal,
            args: std::vec::Vec<u8>,
        ) -> $method_name::Response {
            let method_name = stringify!($method_name);

            $crate::client::pocket::execute_update_encoded_args(
                pic,
                sender,
                canister_id,
                method_name,
                args,
            )
        }
    };
}

#[macro_export]
macro_rules! generate_pocket_query_call_encoded_args {
    ($method_name:ident) => {
        #[allow(dead_code)]
        pub fn $method_name(
            pic: &pocket_ic::PocketIc,
            sender: candid::Principal,
            canister_id: candid::Principal,
            args: &$method_name::Args,
        ) -> $method_name::Response {
            let method_name = stringify!($method_name);

            $crate::client::pocket::generate_pocket_query_call(
                pic,
                sender,
                canister_id,
                method_name,
                args,
            )
        }
    };
}
