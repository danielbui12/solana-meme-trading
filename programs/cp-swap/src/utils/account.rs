use anchor_lang::__private::CLOSED_ACCOUNT_DISCRIMINATOR;
use anchor_lang::prelude::*;
use std::io::{Cursor, Write};
use std::ops::DerefMut;

pub fn close_token_account<'a>(
    authority: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    program: &AccountInfo<'a>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    anchor_spl::token::close_account(CpiContext::new_with_signer(
        program.to_account_info(),
        anchor_spl::token::CloseAccount {
            account: account.to_account_info(),
            destination: destination.to_account_info(),
            authority: authority.to_account_info(),
        },
        signer_seeds,
    ))
}

pub fn close_account<'a>(account: &AccountInfo<'a>, destination: &AccountInfo<'a>) -> Result<()> {
    let dest_starting_lamports = destination.lamports();
    let close_account = account.to_account_info();
    **destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(close_account.lamports())
        .unwrap();
    **close_account.lamports.borrow_mut() = 0;
    let mut data = close_account.try_borrow_mut_data()?;
    for byte in data.deref_mut().iter_mut() {
        *byte = 0;
    }
    let dst: &mut [u8] = &mut data;
    let mut cursor: Cursor<&mut [u8]> = Cursor::new(dst);
    cursor.write_all(&CLOSED_ACCOUNT_DISCRIMINATOR).unwrap();
    Ok(())
}
