/// 配置模块
///
/// WebAssembly 环境下的配置管理
pub struct Config;

impl Config {
    /// 获取 API 基础 URL
    ///
    /// 通过编译时环境变量 `API_BASE_URL` 配置
    /// 默认值: /
    ///
    /// 设置方式:
    /// - 开发环境: 在构建时通过 `trunk` 的 `--env` 参数传递
    /// - 生产环境: 必须通过环境变量配置
    pub fn api_base_url() -> &'static str {
        option_env!("API_BASE_URL").unwrap_or("/")
    }
}
