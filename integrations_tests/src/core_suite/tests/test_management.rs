use crate::client::core_nft::{
    init_upload,
    store_chunk,
    finalize_upload,
    cancel_upload,
    delete_file,
};
use candid::Principal;
use candid::{ Encode, Decode, CandidType, Nat };

use reqwest::blocking::Client;
use reqwest::blocking::ClientBuilder;
use reqwest::blocking::Response;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_api_canister::finalize_upload;
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use sha2::{ Sha256, Digest };
use types::HttpResponse;

use crate::core_suite::setup::setup::TestEnv;
use crate::{ core_suite::setup::default_test_setup, utils::tick_n_blocks };
use std::fs::File;
use std::io::Read;
use std::net::{ IpAddr, Ipv4Addr, SocketAddr };
use std::path::Path;

#[test]
fn test_storage_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(resp) => {
            println!("finalize_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("finalize_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    let endpoint = pic.make_live(None);
    let mut url = endpoint.clone();
    let gateway_host = endpoint.host().unwrap();
    let host = format!("{}.raw.{}", collection_canister_id, gateway_host);

    let client = ClientBuilder::new()
        .resolve(
            "uqqxf-5h777-77774-qaaaa-cai.raw.localhost",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
        )
        .resolve(
            "uxrrr-q7777-77774-qaaaq-cai.raw.localhost",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
        )
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    url.set_host(Some(&host)).unwrap();
    url.set_path("/test.png");

    let mut received_bytes = Vec::new();
    let mut start = 0;
    let mut chunk_size = 1024 * 1024; // Default chunk size
    let max_retries = 5;

    loop {
        // Initial request to get the chunk size from the headers
        println!("url: {:?}", url);
        let initial_res = client.get(url.clone()).send().unwrap();
        if initial_res.status() == 307 {
            if let Some(location) = initial_res.headers().get("location") {
                let location_str = location.to_str().unwrap();
                let sub_canister_id = location_str
                    .split('.')
                    .next()
                    .unwrap()
                    .split('/')
                    .last()
                    .unwrap();
                let host = format!("{}.raw.{}", sub_canister_id, gateway_host);

                url.set_host(Some(&host)).unwrap();
                continue; // Refaire la requête avec la nouvelle URL
            }
        } else if initial_res.status() == 206 {
            if let Some(content_range) = initial_res.headers().get("content-range") {
                let content_range_str = content_range.to_str().unwrap();
                if let Some(range) = content_range_str.split('/').next() {
                    println!("content-range: {:?}", range);
                    let parts: Vec<&str> = range.split(' ').nth(1).unwrap().split('-').collect();
                    println!("parts: {:?}", parts);
                    if parts.len() == 2 {
                        let start_range: usize = parts[0].parse().unwrap();
                        let end_range: usize = parts[1].parse().unwrap();
                        chunk_size = end_range - start_range + 1;
                    }
                }
            }
            break; // Sortir de la boucle si la réponse est 206
        } else {
            panic!("Failed to get initial response: {:?}", initial_res);
        }
    }

    while start < buffer.len() {
        let end = (start + chunk_size - 1).min(buffer.len() - 1);
        let range_header = format!("bytes={}-{}", start, end);

        let res = client.get(url.clone()).header("Range", range_header.clone()).send();

        match res {
            Ok(mut response) => {
                if response.status() == 206 {
                    let mut chunk = Vec::new();
                    response.copy_to(&mut chunk).unwrap();
                    received_bytes.extend_from_slice(&chunk);
                    start += chunk_size;
                }
            }
            Err(e) => {
                panic!("Failed to get response after {} retries: {:?}", max_retries, e);
            }
        }
    }

    pic.stop_live();

    // Check file size
    assert_eq!(
        buffer.len(),
        received_bytes.len(),
        "Uploaded image size does not match the original image size"
    );

    // Check file content
    assert_eq!(
        buffer,
        received_bytes,
        "Uploaded image content does not match the original image content"
    );
}

#[test]
fn test_duplicate_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    // First upload attempt
    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(resp) => {
            println!("finalize_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("finalize_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    // Second upload attempt with the same file
    let init_upload_resp_2 = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    match init_upload_resp_2 {
        Ok(_) => {
            println!("Duplicate upload should not be allowed");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on duplicate upload: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_duplicate_chunk_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        // Attempt to upload the same chunk again
        let duplicate_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        match duplicate_chunk_resp {
            Ok(_) => {
                println!("Duplicate chunk upload should not be allowed");
                assert!(false);
            }
            Err(e) => {
                println!("Expected error on duplicate chunk upload: {:?}", e);
                assert!(true);
            }
        }

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(resp) => {
            println!("finalize_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("finalize_upload_resp error: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_finalize_upload_missing_chunk() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    // Upload all chunks except the last one
    while offset < buffer.len() - (chunk_size as usize) {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    // Attempt to finalize upload with a missing chunk
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed with missing chunk");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on finalize upload with missing chunk: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_cancel_upload() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test_cancel.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test_cancel.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    match init_upload_resp {
        Ok(resp) => {
            println!("init_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("init_upload_resp error: {:?}", e);
        }
    }

    let cancel_upload_resp = cancel_upload(
        pic,
        controller,
        collection_canister_id,
        &(cancel_upload::Args {
            file_path: "/test_cancel.png".to_string(),
        })
    );

    match cancel_upload_resp {
        Ok(resp) => {
            println!("cancel_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("cancel_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    // Attempt to finalize the canceled upload
    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(_) => {
            println!("Finalize upload should not be allowed for a canceled upload");
            assert!(false);
        }
        Err(e) => {
            println!("Expected error on finalize upload for a canceled upload: {:?}", e);
            assert!(true);
        }
    }
}

#[test]
fn test_delete_file() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let file_path = Path::new("./src/storage_suite/assets/test.png");
    let mut file = File::open(&file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let file_size = buffer.len() as u64;

    // Calculate SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let file_hash = hasher.finalize();

    let file_type = "image/png".to_string();
    let media_hash_id = "test_delete.png".to_string();

    let init_upload_resp = init_upload(
        pic,
        controller,
        collection_canister_id,
        &(init_upload::Args {
            file_path: "/test_delete.png".to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size,
            chunk_size: None,
        })
    );

    let mut offset = 0;
    let chunk_size = 1024 * 1024;
    let mut chunk_index = 0;

    while offset < buffer.len() {
        let chunk = &buffer[offset..(offset + (chunk_size as usize)).min(buffer.len())];
        let store_chunk_resp = store_chunk(
            pic,
            controller,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: "/test.png".to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            })
        );

        offset += chunk_size as usize;
        chunk_index += 1;
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        controller,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: "/test.png".to_string(),
        })
    );

    match finalize_upload_resp {
        Ok(resp) => {
            println!("finalize_upload_resp: {:?}", resp);
        }
        Err(e) => {
            println!("finalize_upload_resp error: {:?}", e);
            assert!(false);
        }
    }

    {
        let endpoint = pic.make_live(None);
        let mut url = endpoint.clone();

        let client = ClientBuilder::new()
            .resolve(
                "uqqxf-5h777-77774-qaaaa-cai.raw.localhost",
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
            )
            .resolve(
                "uxrrr-q7777-77774-qaaaq-cai.raw.localhost",
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
            )
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let gateway_host = endpoint.host().unwrap();
        let host = format!("{}.raw.{}", collection_canister_id, gateway_host);
        url.set_host(Some(&host)).unwrap();
        url.set_path("/test_delete.png");

        // Initial request to get the chunk size from the headers
        let initial_res = client.get(url.clone()).send().unwrap();

        match initial_res {
            response => {
                println!("initial_res: {:?}", response);

                if response.status().is_success() || response.status().is_redirection() {
                    println!("File is accessible");
                } else {
                    panic!("File should be accessible");
                }
            }
        }

        pic.stop_live();
    }

    let delete_file_resp = delete_file(
        pic,
        controller,
        collection_canister_id,
        &(delete_file::Args {
            file_path: "/test_delete.png".to_string(),
        })
    );

    match delete_file_resp {
        Ok(resp) => {
            println!("delete_file_resp: {:?}", resp);
        }
        Err(e) => {
            println!("delete_file_resp error: {:?}", e);
            assert!(false);
        }
    }

    let endpoint = pic.make_live(None);
    let mut url = endpoint.clone();

    // Attempt to get the deleted file
    let client = ClientBuilder::new()
        .resolve(
            "uqqxf-5h777-77774-qaaaa-cai.raw.localhost",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
        )
        .resolve(
            "uxrrr-q7777-77774-qaaaq-cai.raw.localhost",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), url.port().unwrap())
        )
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let gateway_host = endpoint.host().unwrap();
    let host = format!("{}.raw.{}", collection_canister_id, gateway_host);
    url.set_host(Some(&host)).unwrap();
    url.set_path("/test_delete.png");

    // Initial request to get the chunk size from the headers
    let initial_res = client.get(url.clone()).send().unwrap();

    match initial_res {
        response => {
            println!("initial_res: {:?}", response);

            if response.status().is_client_error() || response.status().is_server_error() {
                println!("File not found or server error");
            } else {
                panic!("File should not be found after deletion");
            }
        }
    }

    pic.stop_live();
}
