//! 进程级共享的 reqwest blocking Client。
//!
//! Client 内部带连接池，构建一次不便宜；原来每次状态探测（2.5s 一轮）
//! 都新建一个，改成 OnceLock 惰性构建后全局复用。超时按请求单独设置
//! （RequestBuilder::timeout 覆盖客户端默认值），与共享不冲突。

use std::sync::OnceLock;

use reqwest::blocking::Client;

pub fn shared_client() -> Result<&'static Client, String> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))?;
    Ok(CLIENT.get_or_init(|| client))
}
