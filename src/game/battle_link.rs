//! The firmware side of Mesh Battles: the receive handler for a heard RESULT.
//! Task 8 rewrites this as the glue that drives `battle::Session`.

use super::battle::BattleResultMsg;

/// Handle a `BattleResultMsg` received on the SHDW channel.
///
/// Called from `fw::mesh::meshcore::push_grp_data` when a `GrpData`
/// packet on the SHDW slot carries `data_type == PET_BATTLE_TYPE`. Only
/// acts if we are the named target.
pub async fn on_battle_result(data: &[u8]) {
    let Some(msg) = BattleResultMsg::from_bytes(data) else {
        return;
    };

    if msg.target_id != super::friends::local_device_id() {
        return;
    }
    if msg.challenger_id == super::friends::local_device_id() {
        return;
    }

    // From our side, the challenger's result is inverted: their win is
    // our loss. This pet did not fight: only the record changes, in the
    // overall tally and in the head-to-head record against the challenger.
    let we_won = !msg.challenger_won;
    super::lifecycle::record_only(we_won);
    super::friends::record_battle_vs(msg.challenger_id, we_won);
    super::show_toast(if we_won {
        super::Toast::BattleWon
    } else {
        super::Toast::BattleLost
    });
}
