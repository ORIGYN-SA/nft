use candid::{CandidType, Deserialize};

pub mod icrc21_canister_call_consent_message {
    use super::*;

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_consent_message_metadata {
        // BCP-47 language tag. See https://www.rfc-editor.org/rfc/bcp/bcp47.txt
        pub language: String,

        // The users local timezone offset in minutes from UTC.
        // Applicable when converting timestamps to human-readable format.
        //
        // If absent in the request, the canister should fallback to the UTC timezone when creating the consent message.
        // If absent in the response, the canister is indicating that the consent message is not timezone sensitive.
        pub utc_offset_minutes: Option<i16>,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub enum icrc21_device_spec {
        // A generic display able to handle large documents and do line wrapping and pagination / scrolling.
        // Text must be Markdown formatted, no external resources (e.g. images) are allowed.
        GenericDisplay,
        // A simple display able to handle multiple fields with a title and content.
        // It's able to do line wrapping and splits fields into multiple pages if they're too long.
        // Text must be plain text without any embedded formatting elements.
        FieldsDisplay,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_consent_message_spec {
        // Metadata of the consent message.
        pub metadata: icrc21_consent_message_metadata,

        // Information about the device responsible for presenting the consent message to the user.
        // If absent in the request, the canister should fallback to one of the values defined in this spec (ICRC-21).
        pub device_spec: Option<icrc21_device_spec>,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_consent_message_request {
        // Method name of the canister call.
        pub method: String,
        // Argument of the canister call.
        pub arg: Vec<u8>,
        // User preferences with regards to the consent message presented to the end-user.
        pub user_preferences: icrc21_consent_message_spec,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_field_display_message {
        pub intent: String,
        pub fields: Vec<(String, String)>,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_consent_message {
        // Message for a generic display able to handle large documents and do proper line wrapping and pagination / scrolling.
        // Uses Markdown formatting, no external resources (e.g. images) are allowed.
        pub generic_display_message: String,
        // Message for a simple display able to handle multiple fields title and content.
        // It's able to do line wrapping and splits fields into multiple pages if they're too long.
        // Uses plain text, without any embedded formatting elements.
        pub fields_display_message: icrc21_field_display_message,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_consent_info {
        // Consent message describing in a human-readable format what the call will do.
        //
        // The message should adhere as close as possible to the user_preferences specified in the consent_message_spec
        // of the icrc21_consent_message_request.
        // If the message is not available for the given user_preferences any fallback message should be used. Providing a
        // message should be preferred over sending an icrc21_error.
        // The metadata must match the consent_message provided.
        //
        // The message should be short and concise.
        // It should only contain information that is:
        // * relevant to the user
        // * relevant given the canister call argument
        //
        // The message must fit the following context shown to
        // the user on the signer UI:
        // ┌─────────────────────────────────┐
        // │  Approve the following action?  │
        // │  ┌───────────────────────────┐  │
        // │  │    <consent_message>      │  │
        // │  └───────────────────────────┘  │
        // │  ┌───────────┐   ┌───────────┐  │
        // │  │  Reject   │   │  Approve  │  │
        // │  └───────────┘   └───────────┘  │
        // └─────────────────────────────────┘
        pub consent_message: icrc21_consent_message,
        // Metadata of the consent_message.
        pub metadata: icrc21_consent_message_metadata,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_error_info {
        // Human readable technical description of the error intended for developers, not the end-user.
        pub description: String,
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub struct icrc21_generic_error {
        // Machine parsable error. Can be chosen by the target canister but should indicate the error category.
        pub error_code: u64,
        // Human readable technical description of the error intended for developers, not the end-user.
        pub description: String,
    }
    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub enum icrc21_error {
        // The canister does not support this call (i.e. it will lead to a rejection or error response).
        // Reasons might be (non-exhaustive list):
        // * the canister call is malformed (e.g. wrong method name, argument cannot be decoded)
        // * the arguments exceed certain bounds
        //
        // The developer should provide more information about the error using the description in icrc21_error_info.
        UnsupportedCanisterCall(icrc21_error_info),

        // The canister cannot produce a consent message for this call.
        // Reasons might be (non-exhaustive list):
        // * it is an internal call not intended for end-users
        // * the canister developer has not yet implemented a consent message for this call
        //
        // The developer should provide more information about the error using the description in icrc21_error_info.
        ConsentMessageUnavailable(icrc21_error_info),

        // The canister did not provide a consent message for because payment was missing or insufficient.
        //
        // This error is used to account for payment extensions to be added in the future:
        // While small consent messages are easy and cheap to provide, this might not generally be the case for all consent
        // messages. To avoid future breaking changes, when introducing a payment flow, this error is already introduced
        // even though there no standardized payment flow yet.
        InsufficientPayment(icrc21_generic_error),

        // Any error not covered by the above variants.
        GenericError(icrc21_error_info),
    }

    #[allow(non_camel_case_types)]
    #[derive(CandidType, Deserialize)]
    pub enum icrc21_consent_message_response {
        // The call is ok, consent message is provided.
        Ok(icrc21_consent_info),
        // The call is not ok, error is provided.
        Err(icrc21_error),
    }

    pub type Args = icrc21_consent_message_request;
    pub type Response = icrc21_consent_message_response;
}
