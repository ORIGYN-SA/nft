use proc_macro::TokenStream;
use syn::{ parse_macro_input, AttributeArgs, ItemFn, Meta, NestedMeta };
use quote::quote;

#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the arguments and function
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    // Extract attempts and interval from the arguments
    let mut attempts = 1;
    let mut interval = 1;

    for meta in args {
        match meta {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
                let ident = nv.path.get_ident().unwrap();
                let value = if let syn::Lit::Int(lit_int) = &nv.lit {
                    lit_int.base10_parse::<u32>().unwrap()
                } else {
                    panic!("Expected integer value")
                };

                if ident == "attempts" {
                    attempts = value;
                } else if ident == "interval" {
                    interval = value;
                }
            }
            _ => panic!("Unsupported attribute format"),
        }
    }

    // Generate the output
    let fn_name = &input_fn.sig.ident;
    let expanded =
        quote! {
        // Original function
        #input_fn

        // Generated code (example: print values)
        fn run_job() {
            println!("Job function: {}", stringify!(#fn_name));
            println!("Attempts: {}", #attempts);
            println!("Interval: {}", #interval);
        }
    };

    TokenStream::from(expanded)
}
