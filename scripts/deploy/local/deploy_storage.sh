#!/usr/bin/env bash

./scripts/build.sh ./src storage_canister
./scripts/generate_did.sh storage_canister 

dfx deploy --network local storage --argument "(variant { Init = record {
    test_mode = true;
    version = record {
     major = 0:nat32;
     minor = 0:nat32;
     patch = 0:nat32;
    };
    commit_hash = \"stagingcommit\";
    authorized_principals = vec { principal \"6i6da-t3dfv-vteyg-v5agl-tpgrm-63p4y-t5nmm-gi7nl-o72zu-jd3sc-7qe\" };
}})" --mode reinstall

