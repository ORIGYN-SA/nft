#!/bin/bash

# Script to deploy an NFT collection on Internet Computer
# Usage: ./deploy_collection.sh

set -e  # Stop script on error

# Colors for messages
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to display colored messages
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Display warning message
display_warning() {
    echo ""
    echo -e "${YELLOW}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║                        WARNING                               ║${NC}"
    echo -e "${YELLOW}╠══════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${YELLOW}║  This script is for TESTING PURPOSES ONLY!                  ║${NC}"
    echo -e "${YELLOW}║                                                              ║${NC}"
    echo -e "${YELLOW}║  - It will deploy to the IC mainnet                        ║${NC}"
    echo -e "${YELLOW}║  - It will consume cycles from your canister               ║${NC}"
    echo -e "${YELLOW}║  - Make sure you have sufficient cycles                    ║${NC}"
    echo -e "${YELLOW}║  - Use only with test canisters or development purposes   ║${NC}"
    echo -e "${YELLOW}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    read -p "Do you understand and want to continue? (y/n): " CONTINUE
    
    if [[ ! $CONTINUE =~ ^[Yy]$ ]]; then
        print_info "Deployment cancelled by user"
        exit 0
    fi
    
    echo ""
}

# Check prerequisites
check_prerequisites() {
    print_info "Checking prerequisites..."
    
    # Check if dfx is installed
    if ! command -v dfx &> /dev/null; then
        print_error "dfx is not installed. Please install it from https://internetcomputer.org/docs/current/developer-docs/setup/install/"
        exit 1
    fi
    
    # Check if default identity exists
    if ! dfx identity whoami &> /dev/null; then
        print_error "No dfx identity configured. Please create an identity with 'dfx identity new <name>'"
        exit 1
    fi
    
    print_success "Prerequisites verified"
}

# Setup environment variables
setup_environment() {
    print_info "Setting up environment variables..."
    
    # Ask for canister ID
    read -p "Enter the canister ID where you want to deploy (e.g., vvvvv-vvvvv-vvvvv-vvvvv-cai): " CANISTER_ID
    
    if [ -z "$CANISTER_ID" ]; then
        print_error "Canister ID is required"
        exit 1
    fi
    
    # Validate canister ID format (basic validation)
    if [[ ! $CANISTER_ID =~ ^[a-z0-9-]+$ ]]; then
        print_error "Invalid canister ID format. Should contain only lowercase letters, numbers, and hyphens"
        exit 1
    fi
    
    # Ask for necessary information
    read -p "Collection name (e.g., MyCollection): " COLLECTION_NAME
    read -p "Collection symbol (e.g., MC): " COLLECTION_SYMBOL
    read -p "Collection description (optional): " COLLECTION_DESCRIPTION
    
    # Use default values if empty
    COLLECTION_NAME=${COLLECTION_NAME:-"MyCollection"}
    COLLECTION_SYMBOL=${COLLECTION_SYMBOL:-"MC"}
    COLLECTION_DESCRIPTION=${COLLECTION_DESCRIPTION:-"A beautiful NFT collection"}
    
    # Get current identity principal
    YOUR_PRINCIPAL_ID=$(dfx identity get-principal)
    IDENTITY_NAME=$(dfx identity whoami)

    # Export variables
    export CANISTER_ID
    export COLLECTION_NAME
    export COLLECTION_SYMBOL
    export COLLECTION_DESCRIPTION
    export YOUR_PRINCIPAL_ID
    export IDENTITY_FILE="$IDENTITY_NAME.pem"

    echo "Identity file: $IDENTITY_FILE"
    echo "Identity name: $IDENTITY_NAME"
    echo "Principal: $YOUR_PRINCIPAL_ID"


    dfx identity export $IDENTITY_NAME > $IDENTITY_FILE

    print_success "Environment variables configured:"
    echo "  Canister ID: $CANISTER_ID"
    echo "  Collection: $COLLECTION_NAME ($COLLECTION_SYMBOL)"
    echo "  Description: $COLLECTION_DESCRIPTION"
    echo "  Principal: $YOUR_PRINCIPAL_ID"
}

# Deploy collection
deploy_collection() {
    print_info "Deploying NFT collection to canister: $CANISTER_ID..."

    # Deploy canister
    if dfx deploy --network ic nft --mode reinstall --argument "(
        variant {
            Init = record {
                permissions = record {
                    user_permissions = vec {
                        record {
                            principal \"$YOUR_PRINCIPAL_ID\";
                            vec {
                                variant { UpdateMetadata };
                                variant { Minting };
                                variant { UpdateCollectionMetadata };
                                variant { UpdateUploads };
                                variant { ManageAuthorities };
                                variant { ReadUploads };
                            };
                        };
                    };
                };
                supply_cap = null;
                tx_window = null;
                test_mode = false;
                default_take_value = null;
                max_canister_storage_threshold = null;
                logo = null;
                permitted_drift = null;
                name = \"$COLLECTION_NAME\";
                description = opt \"$COLLECTION_DESCRIPTION\";
                version = record {
                    major = 1 : nat32;
                    minor = 1 : nat32;
                    patch = 1 : nat32;
                };
                max_take_value = null;
                max_update_batch_size = null;
                max_query_batch_size = null;
                commit_hash = \"$(git rev-parse HEAD 2>/dev/null || echo 'aaa')\";
                max_memo_size = null;
                atomic_batch_transfers = null;
                collection_metadata = vec {};
                symbol = \"$COLLECTION_SYMBOL\";
                approval_init = record {
                    max_approvals_per_token_or_collection = null;
                    max_revoke_approvals = null;
                };
            }
        }
    )"; then
        print_success "Collection deployed successfully!"
    else
        print_error "Deployment failed!"
    fi
}

# Update canister_ids.json
update_canister_ids() {
    print_info "Updating canister_ids.json file..."
    
    # Create or update canister_ids.json
    cat > canister_ids.json << EOF
{
  "nft": {
    "ic": "$CANISTER_ID"
  }
}
EOF
    
    print_success "canister_ids.json file updated"
}

# Build CLI tool
build_cli_tool() {
    print_info "Building CLI tool..."
    
    cd ../cmdline
    cargo build --release
    
    if [ -f "../target/release/origyn_icrc7_cmdlinetools" ]; then
        print_success "CLI tool built successfully"
    else
        print_error "Failed to build CLI tool"
        exit 1
    fi
    
    cd ../example
}

# Test file upload
test_upload() {
    print_info "Testing file upload..."
    
    # Check if test file exists
    if [ -f "origynlogo.png" ]; then
        ../target/release/origyn_icrc7_cmdlinetools \
            --network ic \
            --identity "$IDENTITY_FILE" \
            --canister "$CANISTER_ID" \
            upload-file ./origynlogo.png origynlogo.png
        
        print_success "Upload test successful"
    else
        print_warning "origynlogo.png file not found, upload test skipped"
    fi
}

# Setup NFT minting
setup_nft_minting() {
    print_info "Setting up NFT minting..."
    
    # Ask if user wants to mint NFTs
    read -p "Do you want to mint some NFTs now? (y/n): " MINT_NFTS
    
    if [[ $MINT_NFTS =~ ^[Yy]$ ]]; then
        # Ask for number of NFTs to mint
        read -p "How many NFTs do you want to mint? (default: 3): " NFT_COUNT
        NFT_COUNT=${NFT_COUNT:-3}
        
        # Ask for NFT type
        echo ""
        echo "NFT types available:"
        echo "1. Simple NFTs with basic metadata"
        echo "2. Interactive NFTs (you'll be prompted for each)"
        echo "3. Predefined collection (e.g., CryptoPunks style)"
        read -p "Choose NFT type (1-3): " NFT_TYPE
        
        case $NFT_TYPE in
            1)
                mint_simple_nfts
                ;;
            2)
                mint_interactive_nfts
                ;;
            3)
                mint_predefined_collection
                ;;
            *)
                print_warning "Invalid choice, skipping NFT minting"
                ;;
        esac
    else
        print_info "NFT minting skipped"
    fi
}

# Mint simple NFTs with basic metadata
mint_simple_nfts() {
    print_info "Minting $NFT_COUNT simple NFTs..."
    
    for i in $(seq 1 $NFT_COUNT); do
        print_info "Minting NFT #$i..."
        
        ../target/release/origyn_icrc7_cmdlinetools \
            --network ic \
            --identity "$IDENTITY_FILE" \
            --canister "$CANISTER_ID" \
            mint \
            --owner "$YOUR_PRINCIPAL_ID" \
            --name "$COLLECTION_NAME #$i" \
            --metadata "description:This is NFT #$i from the $COLLECTION_NAME collection" \
            --metadata "rarity:Common" \
            --metadata "edition:$i" \
            --metadata "total_supply:$NFT_COUNT" \
            --memo "Minted via deployment script"
        
        print_success "NFT #$i minted successfully"
    done
}

# Mint interactive NFTs
mint_interactive_nfts() {
    print_info "Minting $NFT_COUNT interactive NFTs..."
    
    for i in $(seq 1 $NFT_COUNT); do
        echo ""
        print_info "Creating NFT #$i..."
        
        read -p "NFT name: " NFT_NAME
        read -p "NFT description: " NFT_DESCRIPTION
        read -p "Rarity (Common/Rare/Epic/Legendary): " NFT_RARITY
        read -p "Special attribute (optional): " NFT_ATTRIBUTE
        
        NFT_NAME=${NFT_NAME:-"$COLLECTION_NAME #$i"}
        NFT_DESCRIPTION=${NFT_DESCRIPTION:-"Interactive NFT #$i"}
        NFT_RARITY=${NFT_RARITY:-"Common"}
        
        ../target/release/origyn_icrc7_cmdlinetools \
            --network ic \
            --identity "$IDENTITY_FILE" \
            --canister "$CANISTER_ID" \
            mint \
            --owner "$YOUR_PRINCIPAL_ID" \
            --name "$NFT_NAME" \
            --metadata "description:$NFT_DESCRIPTION" \
            --metadata "rarity:$NFT_RARITY" \
            --metadata "edition:$i" \
            --metadata "total_supply:$NFT_COUNT" \
            --memo "Interactive NFT #$i"
        
        if [ ! -z "$NFT_ATTRIBUTE" ]; then
            ../target/release/origyn_icrc7_cmdlinetools \
                --network ic \
                --identity "$IDENTITY_FILE" \
                --canister "$CANISTER_ID" \
                mint \
                --owner "$YOUR_PRINCIPAL_ID" \
                --name "$NFT_NAME" \
                --metadata "special:$NFT_ATTRIBUTE"
        fi
        
        print_success "Interactive NFT #$i minted successfully"
    done
}

# Mint predefined collection (CryptoPunks style)
mint_predefined_collection() {
    print_info "Minting predefined collection ($NFT_COUNT NFTs)..."
    
    # Predefined attributes for variety
    BACKGROUNDS=("Blue" "Purple" "Green" "Red" "Yellow" "Orange" "Pink" "Brown")
    SKIN_TONES=("Light" "Dark" "Medium" "Pale" "Tan")
    EYES=("Blue" "Green" "Brown" "Hazel" "Gray")
    HAIR=("Blonde" "Brown" "Black" "Red" "Gray" "Bald")
    ACCESSORIES=("Hat" "Glasses" "Earring" "Necklace" "Scarf" "None")
    
    for i in $(seq 1 $NFT_COUNT); do
        print_info "Minting NFT #$i..."
        
        # Generate random attributes
        BACKGROUND=${BACKGROUNDS[$((RANDOM % ${#BACKGROUNDS[@]}))]}
        SKIN_TONE=${SKIN_TONES[$((RANDOM % ${#SKIN_TONES[@]}))]}
        EYE_COLOR=${EYES[$((RANDOM % ${#EYES[@]}))]}
        HAIR_COLOR=${HAIR[$((RANDOM % ${#HAIR[@]}))]}
        ACCESSORY=${ACCESSORIES[$((RANDOM % ${#ACCESSORIES[@]}))]}
        
        # Determine rarity based on attributes
        RARITY="Common"
        if [[ "$ACCESSORY" != "None" ]]; then
            RARITY="Rare"
        fi
        if [[ "$BACKGROUND" == "Purple" && "$ACCESSORY" != "None" ]]; then
            RARITY="Epic"
        fi
        if [[ "$BACKGROUND" == "Purple" && "$ACCESSORY" == "Hat" && "$HAIR_COLOR" == "Red" ]]; then
            RARITY="Legendary"
        fi
        
        ../target/release/origyn_icrc7_cmdlinetools \
            --network ic \
            --identity "$IDENTITY_FILE" \
            --canister "$CANISTER_ID" \
            mint \
            --owner "$YOUR_PRINCIPAL_ID" \
            --name "$COLLECTION_NAME #$i" \
            --metadata "description:A unique character from the $COLLECTION_NAME collection" \
            --metadata "rarity:$RARITY" \
            --metadata "background:$BACKGROUND" \
            --metadata "skin_tone:$SKIN_TONE" \
            --metadata "eye_color:$EYE_COLOR" \
            --metadata "hair_color:$HAIR_COLOR" \
            --metadata "accessory:$ACCESSORY" \
            --metadata "edition:$i" \
            --metadata "total_supply:$NFT_COUNT" \
            --memo "Predefined collection NFT #$i"
        
        print_success "NFT #$i minted successfully (Rarity: $RARITY)"
    done
}

# Display final information
show_final_info() {
    print_success "=== DEPLOYMENT COMPLETED ==="
    echo ""
    echo "Your collection information:"
    echo "  Name: $COLLECTION_NAME"
    echo "  Symbol: $COLLECTION_SYMBOL"
    echo "  Canister ID: $CANISTER_ID"
    echo "  Principal: $YOUR_PRINCIPAL_ID"
    echo ""
    echo "Created files:"
    echo "  - canister_ids.json (updated)"
    echo "  - $IDENTITY_FILE (PEM identity)"
    echo ""
    echo "Environment variables to use:"
    echo "  export CANISTER_ID=\"$CANISTER_ID\""
    echo "  export YOUR_PRINCIPAL_ID=\"$YOUR_PRINCIPAL_ID\""
    echo "  export COLLECTION_NAME=\"$COLLECTION_NAME\""
    echo "  export COLLECTION_SYMBOL=\"$COLLECTION_SYMBOL\""
    echo "  export IDENTITY_FILE=\"$IDENTITY_FILE\""
    echo ""
    echo "Useful commands:"
    echo "  # Check total supply"
    echo "  dfx canister call nft --network ic icrc7_total_supply '()'"
    echo ""
    echo "  # Check token metadata"
    echo "  dfx canister call nft --network ic icrc7_token_metadata '(vec { 1 })'"
    echo ""
    echo "  # Mint additional NFTs"
    echo "  ../target/release/origyn_icrc7_cmdlinetools \\"
    echo "    --network ic \\"
    echo "    --identity \$IDENTITY_FILE \\"
    echo "    --canister \$CANISTER_ID \\"
    echo "    mint \\"
    echo "    --owner \$YOUR_PRINCIPAL_ID \\"
    echo "    --name \"My first NFT\" \\"
    echo "    --interactive"
    echo ""
}

# Main function
main() {
    echo "=== NFT COLLECTION DEPLOYMENT SCRIPT ==="
    echo ""
    
    display_warning
    check_prerequisites
    setup_environment
    update_canister_ids
    deploy_collection
    build_cli_tool
    test_upload
    setup_nft_minting
    show_final_info
}

# Execute main function
main "$@"
