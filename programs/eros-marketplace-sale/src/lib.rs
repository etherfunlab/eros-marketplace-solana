pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Ca8tTnDxUcXd1FKDaCc1x8m8faEU6NB3jfLhDNvrZK8a");

#[program]
pub mod eros_marketplace_sale {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }
}
