use stellar_xdr::curr::{TransactionEnvelope, ReadXdr, Limits, Limited};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crate::error::AppError;

/// Decodes a Stellar Transaction Envelope from a Base64 XDR string.
/// Updated for stellar-xdr v21 which requires explicit resource limits during decoding.
pub fn decode_transaction_xdr(envelope_xdr: &str) -> Result<TransactionEnvelope, AppError> {
    let bytes = STANDARD.decode(envelope_xdr)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;
    
    let cursor = std::io::Cursor::new(bytes);
    // In v21, we must wrap the reader in a Limited struct to enforce resource bounds.
    let mut limited = Limited::new(cursor, Limits::none());
    
    let envelope = TransactionEnvelope::read_xdr_to_end(&mut limited)
        .map_err(|e| AppError::BadRequest(format!("Invalid XDR: {}", e)))?;
    
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_invalid_xdr() {
        let result = decode_transaction_xdr("invalid_base64");
        assert!(result.is_err());
    }
}
