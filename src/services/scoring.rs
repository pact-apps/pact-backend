use sqlx::PgPool;

/// Update commitment score setelah challenge settle
///
/// Scoring rules (sesuai PDF):
/// - Complete challenge: +10 points, streak +1
/// - Fail challenge: -15 points, streak reset
/// - Disputed & failed: -20 points
/// - No submission (auto-fail): -25 points
pub async fn update_score_on_complete(
    pool: &PgPool,
    wallet: &str,
    stake_lamports: i64,
    earned_lamports: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO commitment_scores (wallet_address, score, challenges_joined, challenges_completed, total_staked_lamports, total_earned_lamports, streak_current, streak_best)
        VALUES ($1, 110, 1, 1, $2, $3, 1, 1)
        ON CONFLICT (wallet_address) DO UPDATE SET
            score = commitment_scores.score + 10,
            challenges_completed = commitment_scores.challenges_completed + 1,
            challenges_joined = commitment_scores.challenges_joined + 1,
            total_staked_lamports = commitment_scores.total_staked_lamports + $2,
            total_earned_lamports = commitment_scores.total_earned_lamports + $3,
            streak_current = commitment_scores.streak_current + 1,
            streak_best = GREATEST(commitment_scores.streak_best, commitment_scores.streak_current + 1),
            updated_at = NOW()
        "#,
    )
    .bind(wallet)
    .bind(stake_lamports)
    .bind(earned_lamports)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_score_on_fail(
    pool: &PgPool,
    wallet: &str,
    stake_lamports: i64,
    was_disputed: bool,
    was_auto_fail: bool,
) -> anyhow::Result<()> {
    let penalty = if was_auto_fail {
        -25
    } else if was_disputed {
        -20
    } else {
        -15
    };

    sqlx::query(
        r#"
        INSERT INTO commitment_scores (wallet_address, score, challenges_joined, challenges_failed, total_staked_lamports, streak_current)
        VALUES ($1, GREATEST(0, 100 + $2), 1, 1, $3, 0)
        ON CONFLICT (wallet_address) DO UPDATE SET
            score = GREATEST(0, commitment_scores.score + $2),
            challenges_failed = commitment_scores.challenges_failed + 1,
            challenges_joined = commitment_scores.challenges_joined + 1,
            total_staked_lamports = commitment_scores.total_staked_lamports + $3,
            streak_current = 0,
            updated_at = NOW()
        "#,
    )
    .bind(wallet)
    .bind(penalty)
    .bind(stake_lamports)
    .execute(pool)
    .await?;

    if was_disputed {
        sqlx::query(
            "UPDATE commitment_scores SET challenges_disputed = challenges_disputed + 1 WHERE wallet_address = $1"
        )
        .bind(wallet)
        .execute(pool)
        .await?;
    }

    Ok(())
}