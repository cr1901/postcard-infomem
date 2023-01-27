#![no_std]

#[macro_export]
macro_rules! include_postcard_infomem {
    ($pim:expr) => {
        include_postcard_infomem!($pim, ".info", INFOMEM);
    };

    ($pim:expr, $sec:literal) => {
        include_postcard_infomem!($pim, $sec, INFOMEM);
    };

    ($pim:expr, $sec:literal, $var_name:ident) => {
        const INFOMEM_REF: &[u8] = include_bytes!($pim);
        const INFOMEM_LEN: usize = INFOMEM_REF.len();

        #[link_section = $sec]
        #[used]
        #[no_mangle]
        static $var_name: [u8; INFOMEM_LEN] = {
            let mut arr = [0; INFOMEM_LEN];
            let mut idx = 0;
            
            while idx < INFOMEM_LEN {
                arr[idx] = INFOMEM_REF[idx];
                idx += 1;
            }

            arr
        };
    };
}


#[cfg(test)]
mod tests {

}
