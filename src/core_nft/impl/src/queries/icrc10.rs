use ic_cdk::query;

pub use crate::types::icrc10;

#[query]
pub fn icrc10_supported_standards() -> icrc10::icrc10_supported_standards::Response {
    vec![
        icrc10::icrc10_supported_standards::SupportedStandard {
            name: "ICRC-37".to_string(),
            url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-37/ICRC-37.md".to_string(),
        },
        icrc10::icrc10_supported_standards::SupportedStandard {
            name: "ICRC-3".to_string(),
            url: "https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-3/README.md"
                .to_string(),
        },
        icrc10::icrc10_supported_standards::SupportedStandard {
            name: "ICRC-7".to_string(),
            url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-7/ICRC-7.md".to_string(),
        },
        icrc10::icrc10_supported_standards::SupportedStandard {
            name: "ICRC-10".to_string(),
            url: "https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-10/ICRC-10.md".to_string(),
        },
        icrc10::icrc10_supported_standards::SupportedStandard {
            name: "ICRC-21".to_string(),
            url: "https://github.com/dfinity/wg-identity-authentication/blob/main/topics/ICRC-21/icrc_21_consent_msg.md".to_string(),
        },
    ]
}
