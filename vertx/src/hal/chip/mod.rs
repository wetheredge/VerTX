#[cfg(not(test))]
select_mod! {
    "chip-esp": esp,
    "chip-rp": rp,
    "simulator": simulator,
}

#[cfg(test)]
mod test;
#[cfg(test)]
pub(crate) use test::*;
