use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use candid::Nat;
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use serde_json::{json, Value};
use std::io::Write;

use crate::prompts::{prompt_bool, prompt_display_type, prompt_input, prompt_optional};

pub fn validate_icrc97_metadata(metadata: &Value) -> Result<()> {
    if !metadata.is_object() {
        return Err(anyhow!("Metadata must be a JSON object"));
    }

    let obj = metadata.as_object().unwrap();

    if let Some(name) = obj.get("name") {
        if !name.is_string() {
            return Err(anyhow!("'name' field must be a string"));
        }
    }

    if let Some(description) = obj.get("description") {
        if !description.is_string() {
            return Err(anyhow!("'description' field must be a string"));
        }
    }

    if let Some(image) = obj.get("image") {
        if !image.is_string() {
            return Err(anyhow!("'image' field must be a string"));
        }
    }

    if let Some(external_url) = obj.get("external_url") {
        if !external_url.is_string() {
            return Err(anyhow!("'external_url' field must be a string"));
        }
    }

    if let Some(attributes) = obj.get("attributes") {
        if !attributes.is_array() {
            return Err(anyhow!("'attributes' field must be an array"));
        }

        for (i, attr) in attributes.as_array().unwrap().iter().enumerate() {
            if !attr.is_object() {
                return Err(anyhow!("Attribute {} must be an object", i));
            }

            let attr_obj = attr.as_object().unwrap();

            if !attr_obj.contains_key("trait_type") {
                return Err(anyhow!("Attribute {} must have 'trait_type' field", i));
            }

            if !attr_obj.get("trait_type").unwrap().is_string() {
                return Err(anyhow!("Attribute {} 'trait_type' must be a string", i));
            }

            if !attr_obj.contains_key("value") {
                return Err(anyhow!("Attribute {} must have 'value' field", i));
            }

            if let Some(display_type) = attr_obj.get("display_type") {
                if !display_type.is_string() {
                    return Err(anyhow!("Attribute {} 'display_type' must be a string", i));
                }
            }
        }
    }

    Ok(())
}

pub fn create_metadata_interactive_hashmap() -> Result<Vec<(String, ICRC3Value)>> {
    println!("=== Interactive Metadata Creation (HashMap Mode) ===");
    let mut metadata = Vec::new();

    loop {
        println!("\n--- Adding metadata entry ---");
        let key = prompt_input("Metadata key (or press Enter to finish)");
        if key.is_empty() {
            break;
        }

        println!("\nValue type options:");
        println!("  1. Text");
        println!("  2. Number (Nat)");
        println!("  3. Integer (Int)");
        println!("  4. Blob (Base64 encoded)");

        let value_type = prompt_input("Choose value type (1-4)");
        let value = match value_type.as_str() {
            "1" => {
                let text = prompt_input("Enter text value");
                ICRC3Value::Text(text)
            }
            "2" => {
                let num_str = prompt_input("Enter number value");
                let num = num_str
                    .parse::<u64>()
                    .map_err(|_| anyhow!("Invalid number: {}", num_str))?;
                ICRC3Value::Nat(Nat::from(num))
            }
            "3" => {
                let int_str = prompt_input("Enter integer value");
                let int = int_str
                    .parse::<i64>()
                    .map_err(|_| anyhow!("Invalid integer: {}", int_str))?;
                ICRC3Value::Int(candid::Int::from(int))
            }
            "4" => {
                let blob_str = prompt_input("Enter blob value (base64 encoded)");
                let blob_bytes = BASE64
                    .decode(&blob_str)
                    .map_err(|_| anyhow!("Invalid base64: {}", blob_str))?;
                ICRC3Value::Blob(serde_bytes::ByteBuf::from(blob_bytes))
            }
            _ => {
                println!("Invalid choice, using text");
                let text = prompt_input("Enter text value");
                ICRC3Value::Text(text)
            }
        };

        metadata.push((key, value));

        if !prompt_bool("Add another metadata entry?") {
            break;
        }
    }

    Ok(metadata)
}

pub fn create_icrc97_metadata_from_url(url: &str) -> Vec<(String, ICRC3Value)> {
    vec![(
        "icrc97:metadata".to_string(),
        ICRC3Value::Array(vec![ICRC3Value::Text(url.to_string())]),
    )]
}

pub fn create_metadata_interactive() -> Result<Value> {
    println!("=== ICRC97 Metadata Creation ===");

    let name = prompt_input("NFT name");
    if name.is_empty() {
        return Err(anyhow!("Name is required"));
    }

    let description = prompt_input("NFT description");
    if description.is_empty() {
        return Err(anyhow!("Description is required"));
    }

    let image = prompt_optional("Image URL");
    let external_url = prompt_optional("External URL");

    let mut metadata = json!({
        "name": name,
        "description": description
    });

    if let Some(img) = image {
        metadata["image"] = json!(img);
    }

    if let Some(ext_url) = external_url {
        metadata["external_url"] = json!(ext_url);
    }

    if prompt_bool("Add attributes?") {
        let mut attributes = Vec::new();

        loop {
            println!("\n--- Adding attribute ---");
            let trait_type = prompt_input("Trait type");
            if trait_type.is_empty() {
                break;
            }

            let value_str = prompt_input("Value");
            let (value, is_number) = if let Ok(num) = value_str.parse::<f64>() {
                (json!(num), true)
            } else {
                (json!(value_str), false)
            };

            let display_type = prompt_display_type(is_number);

            let mut attr = json!({
                "trait_type": trait_type,
                "value": value
            });

            if let Some(display) = display_type {
                attr["display_type"] = json!(display);
            }

            attributes.push(attr);

            if !prompt_bool("Add another attribute?") {
                break;
            }
        }

        if !attributes.is_empty() {
            metadata["attributes"] = json!(attributes);
        }
    }

    Ok(metadata)
}

pub fn create_icrc97_metadata(
    name: &str,
    description: &str,
    image_url: Option<&str>,
    external_url: Option<&str>,
    attributes: Vec<(String, Value, Option<String>)>,
) -> Value {
    let mut metadata = json!({
        "name": name,
        "description": description
    });

    if let Some(image) = image_url {
        metadata["image"] = json!(image);
    }

    if let Some(external) = external_url {
        metadata["external_url"] = json!(external);
    }

    if !attributes.is_empty() {
        let attrs: Vec<Value> = attributes
            .into_iter()
            .map(|(trait_type, value, display_type)| {
                let mut attr = json!({
                    "trait_type": trait_type,
                    "value": value
                });
                if let Some(display) = display_type {
                    attr["display_type"] = json!(display);
                }
                attr
            })
            .collect();
        metadata["attributes"] = json!(attrs);
    }

    metadata
}
