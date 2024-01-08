use super::{Domain, IntoTerm, Proof, Validation};
use crate::{
	did::{Did, Schema},
	error::AttTrError,
	term::Term,
	utils::address_from_ecdsa_key,
};
use secp256k1::{
	ecdsa::{RecoverableSignature, RecoveryId},
	generate_keypair,
	rand::thread_rng,
	Message, PublicKey, Secp256k1,
};
use serde_derive::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Deserialize, Serialize, Clone)]
struct DomainTrust {
	scope: Domain,
	level: f32,
	reason: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct CredentialSubject {
	id: String,
	trustworthiness: Vec<DomainTrust>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrustSchema {
	#[serde(alias = "type")]
	kind: String,
	issuer: String,
	credential_subject: CredentialSubject,
	proof: Proof,
}

#[cfg(test)]
impl TrustSchema {
	fn new(id: String, trust_arc: DomainTrust) -> Self {
		let did = Did::parse_snap(id.clone()).unwrap();
		let mut keccak = Keccak256::default();
		keccak.update(&did.key);
		keccak.update(&[trust_arc.scope.clone().into()]);
		keccak.update(trust_arc.level.to_be_bytes());
		let digest = keccak.finalize();

		let message = Message::from_digest_slice(digest.as_ref()).unwrap();

		let rng = &mut thread_rng();
		let (sk, pk) = generate_keypair(rng);
		let secp = Secp256k1::new();
		let res = secp.sign_ecdsa_recoverable(&message, &sk);
		let (rec_id, sig_bytes) = res.serialize_compact();
		let rec_id_i32 = rec_id.to_i32();

		let mut bytes = Vec::new();
		bytes.extend_from_slice(&sig_bytes);
		bytes.push(rec_id_i32.to_le_bytes()[0]);
		let encoded_sig = hex::encode(bytes);

		let kind = "AuditReportApproveCredential".to_string();
		let address = address_from_ecdsa_key(&pk);
		let issuer = format!("did:pkh:eth:{}", hex::encode(address));
		let cs = CredentialSubject { id, trustworthiness: vec![trust_arc] };
		let proof = Proof { signature: encoded_sig };

		TrustSchema { kind, issuer, credential_subject: cs, proof }
	}
}

impl Validation for TrustSchema {
	fn get_trimmed_signature(&self) -> String {
		self.proof.get_signature().trim_start_matches("0x").to_owned()
	}

	fn get_message(&self) -> Result<Vec<u8>, AttTrError> {
		let did = Did::parse_snap(self.credential_subject.id.clone())?;

		let mut bytes = Vec::new();
		bytes.extend_from_slice(&did.key);
		let arc = self.credential_subject.trustworthiness[0].clone();
		bytes.push(arc.scope.into());
		bytes.extend_from_slice(&arc.level.to_be_bytes());

		Ok(bytes)
	}
}

impl IntoTerm for TrustSchema {
	fn into_term(self) -> Result<Term, AttTrError> {
		let pk = self.validate()?;

		let from_address = address_from_ecdsa_key(&pk);
		let from_did: String = Did::new(Schema::PkhEth, from_address).into();
		let weight = 50;
		let domain = 1;

		Ok(Term::new(
			from_did, self.credential_subject.id, weight, domain, true,
		))
	}
}

// #[cfg(test)]
// mod test {
// 	use crate::{
// 		did::Did,
// 		schemas::{approve::CredentialSubject, Proof, Validation},
// 		utils::address_from_ecdsa_key,
// 	};

// 	use super::AuditApproveSchema;
// 	use secp256k1::{generate_keypair, rand::thread_rng, Message, Secp256k1};
// 	use sha3::{Digest, Keccak256};

// 	#[test]
// 	fn should_validate_audit_approve_schema() {
// 		let did_string = "snap://90f8bf6a47".to_owned();
// 		let did = Did::parse_snap(did_string.clone()).unwrap();

// 		let mut keccak = Keccak256::default();
// 		keccak.update(&did.key);
// 		let digest = keccak.finalize();

// 		let message = Message::from_digest_slice(digest.as_ref()).unwrap();

// 		let rng = &mut thread_rng();
// 		let (sk, pk) = generate_keypair(rng);
// 		let secp = Secp256k1::new();
// 		let res = secp.sign_ecdsa_recoverable(&message, &sk);
// 		let (rec_id, sig_bytes) = res.serialize_compact();
// 		let rec_id = rec_id.to_i32().to_le_bytes()[0];

// 		let mut bytes = Vec::new();
// 		bytes.extend_from_slice(&sig_bytes);
// 		bytes.push(rec_id);
// 		let sig_string = hex::encode(bytes);

// 		let kind = "AuditReportApproveCredential".to_string();
// 		let addr = address_from_ecdsa_key(&pk);
// 		let issuer = format!("did:pkh:eth:{}", hex::encode(addr));
// 		let cs = CredentialSubject { id: did_string };
// 		let proof = Proof { signature: sig_string };

// 		let aa_schema = AuditApproveSchema { kind, issuer, credential_subject: cs, proof };

// 		let rec_pk = aa_schema.validate().unwrap();

// 		assert_eq!(rec_pk, pk);
// 	}
// }
