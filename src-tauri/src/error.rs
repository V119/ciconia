use serde::{Serialize, Serializer};

// 1. 定义一个元组结构体包裹 anyhow::Error
#[derive(Debug)]
pub struct CommandError(anyhow::Error);

// 2. 实现 From trait，允许直接使用 `?` 操作符将 anyhow::Error 转换过来
impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        CommandError(err)
    }
}

// 3. 实现 Serialize，将错误转化为字符串传给前端
impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 这里可以选择只返回 error message，或者包含 chain
        serializer.serialize_str(&self.0.to_string())
    }
}

// 4. 定义一个简化的 Result 类型别名
pub type CommandResult<T> = Result<T, CommandError>;
