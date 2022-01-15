fn is_ipfs<T: ToString>(url: T) -> bool {
    (&url.to_string()[..7] == "ipfs://")
}

pub fn to_ipfs<T: ToString>(url: T) -> String {
    if is_ipfs(url.to_string()) {
        format!("https//ipfs.io/{}", &url.to_string()[7..url.to_string().len()])
    } else {
        url.to_string()
    }
}