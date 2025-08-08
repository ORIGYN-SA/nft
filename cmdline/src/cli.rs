use clap::{arg, value_parser, ArgAction, Command};

pub fn build_cli() -> Command {
    Command::new("ICRC7 NFT Tool")
        .version("1.0")
        .author("Gautier Wojda <gautier.wojda@bity.com>")
        .about("Complete tool for ICRC7 NFT management")
        .arg(
            arg!(-n --network <NETWORK> "Network to connect to")
                .value_parser(["local", "ic"])
                .default_value("local")
        )
        .arg(
            arg!(-i --identity <IDENTITY> "Path to identity PEM file")
                .required(true)
        )
        .arg(
            arg!(-c --canister <CANISTER_ID> "Target canister ID")
                .required(true)
        )
        .subcommand(
            Command::new("upload-file")
                .about("Upload a file to the canister")
                .arg(arg!(<file_path> "Path to the file to upload"))
                .arg(arg!(<destination_path> "Destination path in the canister (e.g. /images/photo.jpg)"))
                .arg(
                    arg!(-s --chunk_size <SIZE> "Chunk size in bytes")
                        .value_parser(value_parser!(u64))
                        .default_value("1048576")
                )
        )
        .subcommand(
            Command::new("validate-metadata")
                .about("Validate ICRC97 JSON metadata file")
                .arg(arg!(<metadata_file> "Path to JSON metadata file"))
        )
        .subcommand(
            Command::new("create-metadata")
                .about("Create ICRC97 metadata interactively or from parameters")
                .arg(arg!(-o --output <FILE> "Output JSON file path").required(true))
                .arg(arg!(-i --interactive "Use interactive mode"))
                .arg(arg!(-n --name <NAME> "NFT name"))
                .arg(arg!(-d --description <DESC> "NFT description"))
                .arg(arg!(--image <URL> "Image URL"))
                .arg(arg!(--external_url <URL> "External URL"))
                .arg(
                    arg!(-a --attribute <ATTR> "Add attribute (format: trait_type:value[:display_type])")
                        .action(ArgAction::Append)
                )
        )
        .subcommand(
            Command::new("upload-metadata")
                .about("Upload JSON metadata file to canister")
                .arg(arg!(<metadata_file> "Path to JSON metadata file"))
                .arg(
                    arg!(-s --chunk_size <SIZE> "Chunk size in bytes")
                        .value_parser(value_parser!(u64))
                        .default_value("1048576")
                )
        )
        .subcommand(
            Command::new("mint")
                .about("Mint an NFT with metadata")
                .arg(arg!(-o --owner <OWNER> "Owner principal").required(true))
                .arg(arg!(-n --name <NAME> "Token name").required(true))
                .arg(arg!(--memo <MEMO> "Optional memo"))
                .arg(
                    arg!(--subaccount <SUBACCOUNT> "Optional subaccount (32 bytes hex)")
                )
                .arg(
                    arg!(--icrc97_url <URL> "ICRC97 metadata URL (creates icrc97:metadata)")
                )
                .arg(
                    arg!(--interactive "Use interactive mode to create metadata")
                )
                .arg(
                    arg!(-m --metadata <KEY_VALUE> "Add metadata entry (format: key:value)")
                        .action(ArgAction::Append)
                )
        )
        .subcommand(
            Command::new("permissions")
                .about("Manage permissions: grant, revoke, list, has")
                .subcommand(
                    Command::new("grant")
                        .about("Grant a permission to a principal")
                        .arg(arg!(--principal <PRINCIPAL> "Target principal").required(true))
                        .arg(arg!(--permission <PERM> "Permission").required(true)),
                )
                .subcommand(
                    Command::new("revoke")
                        .about("Revoke a permission from a principal")
                        .arg(arg!(--principal <PRINCIPAL> "Target principal").required(true))
                        .arg(arg!(--permission <PERM> "Permission").required(true)),
                )
                .subcommand(
                    Command::new("list")
                        .about("List permissions for a principal")
                        .arg(arg!(--principal <PRINCIPAL> "Target principal").required(true)),
                )
                .subcommand(
                    Command::new("has")
                        .about("Check whether a principal has a permission")
                        .arg(arg!(--principal <PRINCIPAL> "Target principal").required(true))
                        .arg(arg!(--permission <PERM> "Permission").required(true)),
                ),
        )
}
