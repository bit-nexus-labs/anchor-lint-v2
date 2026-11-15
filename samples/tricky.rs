// Cases where the first (text-matching) version was wrong. Expected findings are listed in SPEC.md.
use anchor_lib::prelude::*;

const FEE: u64 = 10;

fn text_and_comments() {
    let s = "a + b and x.unwrap()";
    let t = 1; // a trailing comment with y.unwrap()
}

fn multiline(x: u64, y: u64) -> u64 {
    x
        + y
}

fn constants() -> u64 {
    FEE + 5
}

fn no_spaces(x: u64, y: u64) -> u64 {
    x+y
}

fn macros(a: u64, b: u64) {
    msg!("sum {}", a + b);
    require!(a + b > 10, ErrorCode::TooBig);
}

#[derive(Accounts)]
pub struct Ctx<'info> {
    pub a: AccountInfo<'info>,
    /// CHECK: validated in the handler
    pub b: AccountInfo<'info>,
}

pub struct NotAnchor<'info> {
    pub c: AccountInfo<'info>,
}

#[cfg(test)]
mod tests {
    fn t() {
        let v = Some(1).unwrap();
        let z = 1 + v;
    }
}
