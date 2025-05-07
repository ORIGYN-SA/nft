#!/usr/bin/env bash

show_help() {
  cat << EOF
Generate the candid file and declarations for the mentioned wasm canister.
Must be run from the repository's root folder.

Usage:
  scripts/generate_did [options] <wasm>

Options:
  -h, --help        Show this message and exit
  -o, --output PATH The path where to write the resulting candid file (Must be a folder)
  -d, --dry-run     Only output the result, without actually writing on disk
EOF
}

if [[ $# -gt 0 ]]; then
  while [[ "$1" =~ ^- && ! "$1" == "--" ]]; do
    case $1 in
      -h | --help )
        show_help
        exit
        ;;
      -o | --output )
        outpath=$2
        ;;
      -d | --dry-run )
        dryrun=1
        ;;
    esac;
    shift;
  done
  if [[ "$1" == '--' ]]; then shift; fi
else
  echo "Error: not enough arguments."
  show_help
  exit 1
fi

output_path=$outpath
echo "output_path: ${output_path}"
if [[ $dryrun -eq 1 ]]; then
  echo -e "This would be written to ${output_path}/${1}.did :\n"
  candid-extractor "$1"
else
  candid-extractor "$1" > $output_path
fi
