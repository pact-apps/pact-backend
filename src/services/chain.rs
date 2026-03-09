use anyhow::{bail, Context};

use crate::config::AppState;
use crate::models::chain::{
    AccountMetaPlan, FinalizePlanRequest, FinalizePlanResponse,
};

pub fn build_finalize_plan(
    state: &AppState,
    request: FinalizePlanRequest,
) -> anyhow::Result<FinalizePlanResponse> {
    if request.participant_accounts.is_empty() {
        bail!("participant_accounts cannot be empty");
    }

    let treasury_token_account = state
        .treasury_token_accounts
        .get(&request.stake_mint)
        .cloned()
        .with_context(|| format!("unsupported stake mint: {}", request.stake_mint))?;

    let mut remaining_accounts =
        Vec::with_capacity(request.participant_accounts.len() * 3);
    let mut account_meta_order = vec![
        AccountMetaPlan {
            role: "platform_config".to_string(),
            pubkey: state.platform_config_pda.clone(),
        },
        AccountMetaPlan {
            role: "challenge".to_string(),
            pubkey: request.challenge.clone(),
        },
        AccountMetaPlan {
            role: "vault".to_string(),
            pubkey: request.vault.clone(),
        },
        AccountMetaPlan {
            role: "treasury_token_account".to_string(),
            pubkey: treasury_token_account.clone(),
        },
        AccountMetaPlan {
            role: "token_program".to_string(),
            pubkey: request.token_program.clone(),
        },
    ];

    for (index, participant) in request.participant_accounts.iter().enumerate() {
        remaining_accounts.push(participant.participant_state.clone());
        remaining_accounts.push(participant.payout_token_account.clone());
        remaining_accounts.push(participant.commitment_profile.clone());

        account_meta_order.push(AccountMetaPlan {
            role: format!("participant_state_{}", index),
            pubkey: participant.participant_state.clone(),
        });
        account_meta_order.push(AccountMetaPlan {
            role: format!("payout_token_account_{}", index),
            pubkey: participant.payout_token_account.clone(),
        });
        account_meta_order.push(AccountMetaPlan {
            role: format!("commitment_profile_{}", index),
            pubkey: participant.commitment_profile.clone(),
        });
    }

    Ok(FinalizePlanResponse {
        program_id: state.program_id.clone(),
        platform_config_pda: state.platform_config_pda.clone(),
        treasury_authority: state.treasury_authority.clone(),
        treasury_token_account,
        platform_fee_bps: state.platform_fee_bps,
        challenge: request.challenge,
        vault: request.vault,
        token_program: request.token_program,
        stake_mint: request.stake_mint,
        challenge_authority: request.challenge_authority,
        remaining_accounts,
        participant_triples: request.participant_accounts,
        account_meta_order,
    })
}
