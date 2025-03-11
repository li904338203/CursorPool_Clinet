use std::path::PathBuf;
use std::process::Command;
use std::fs;
use zip::write::FileOptions;
use std::io::prelude::*;

pub struct UpdateBlocker;

impl UpdateBlocker {
    pub fn new() -> Self {
        UpdateBlocker
    }

    /// 禁用 Cursor 自动更新
    pub fn disable_auto_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        // 如果是目录, 先备份
        if updater_path.is_dir() {
            self.backup_updater_dir(updater_path)?;
        }

        match std::env::consts::OS {
            "windows" => self.disable_windows_update(updater_path),
            "macos" => self.disable_macos_update(),
            "linux" => self.disable_linux_update(updater_path),
            _ => Err("不支持的操作系统".to_string()),
        }
    }

    /// 备份更新器目录
    fn backup_updater_dir(&self, updater_path: &PathBuf) -> Result<(), String> {
        if !updater_path.is_dir() {
            return Ok(());
        }

        let parent = updater_path.parent()
            .ok_or("无法获取父目录")?;
        let backup_path = parent.join("cursor-updater.zip");

        let file = fs::File::create(&backup_path)
            .map_err(|e| format!("创建备份文件失败: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        self.add_dir_to_zip(&mut zip, updater_path, updater_path, options)?;
        zip.finish().map_err(|e| format!("完成ZIP文件失败: {}", e))?;

        Ok(())
    }

    /// 递归添加目录内容到ZIP
    fn add_dir_to_zip(
        &self,
        zip: &mut zip::ZipWriter<fs::File>,
        base_path: &PathBuf,
        dir_path: &PathBuf,
        options: FileOptions,
    ) -> Result<(), String> {
        for entry in fs::read_dir(dir_path)
            .map_err(|e| format!("读取目录失败: {}", e))? {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            let name = path.strip_prefix(base_path)
                .map_err(|e| format!("处理路径前缀失败: {}", e))?
                .to_string_lossy();

            if path.is_file() {
                zip.start_file(name.into_owned(), options)
                    .map_err(|e| format!("添加文件到ZIP失败: {}", e))?;
                let mut f = fs::File::open(&path)
                    .map_err(|e| format!("打开文件失败: {}", e))?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)
                    .map_err(|e| format!("读取文件失败: {}", e))?;
                zip.write_all(&buffer)
                    .map_err(|e| format!("写入ZIP失败: {}", e))?;
            } else if path.is_dir() {
                self.add_dir_to_zip(zip, base_path, &path, options)?;
            }
        }
        Ok(())
    }

    /// Windows 平台禁用更新
    fn disable_windows_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        // 删除现有文件/目录
        if updater_path.exists() {
            if updater_path.is_dir() {
                fs::remove_dir_all(updater_path)
                    .map_err(|e| format!("删除目录失败: {}", e))?;
            } else {
                fs::remove_file(updater_path)
                    .map_err(|e| format!("删除文件失败: {}", e))?;
            }
        }

        // 创建空文件
        fs::File::create(updater_path)
            .map_err(|e| format!("创建文件失败: {}", e))?;

        // 获取当前用户名
        let output = Command::new("powershell")
            .args(["-Command", "$env:USERNAME"])
            .output()
            .map_err(|e| format!("获取用户名失败: {}", e))?;
        
        let username = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 移除所有现有权限并只设置当前用户的读取权限
        let output = Command::new("icacls")
            .args([
                updater_path.to_str().unwrap(),
                "/reset"
            ])
            .output()
            .map_err(|e| format!("重置权限失败: {}", e))?;
        if !output.status.success() {
            return Err(format!("重置权限失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // 禁用继承
        let output = Command::new("icacls")
            .args([
                updater_path.to_str().unwrap(),
                "/inheritance:d"
            ])
            .output()
            .map_err(|e| format!("禁用继承失败: {}", e))?;
        if !output.status.success() {
            return Err(format!("禁用继承失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // 移除所有权限
        let output = Command::new("icacls")
            .args([
                updater_path.to_str().unwrap(),
                "/remove", "Everyone",
                "/remove", "Users",
                "/remove", "Authenticated Users"
            ])
            .output()
            .map_err(|e| format!("移除权限失败: {}", e))?;
        if !output.status.success() {
            return Err(format!("移除权限失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // 设置当前用户只读权限
        let output = Command::new("icacls")
            .args([
                updater_path.to_str().unwrap(),
                "/grant:r",
                &format!("{}:(R)", username)
            ])
            .output()
            .map_err(|e| format!("设置权限失败: {}", e))?;
        if !output.status.success() {
            return Err(format!("设置用户权限失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // 设置系统权限（可能需要保留）
        let output = Command::new("icacls")
            .args([
                updater_path.to_str().unwrap(),
                "/grant:r",
                "SYSTEM:(R)"
            ])
            .output()
            .map_err(|e| format!("设置系统权限失败: {}", e))?;
        if !output.status.success() {
            return Err(format!("设置系统权限失败: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // 设置只读属性
        Command::new("attrib")
            .args(["+r", "+s", "+h", updater_path.to_str().unwrap()])  // 添加系统和隐藏属性
            .output()
            .map_err(|e| format!("设置文件属性失败: {}", e))?;

        // 验证权限
        Command::new("icacls")
            .arg(updater_path.to_str().unwrap())
            .output()
            .map_err(|e| format!("验证权限失败: {}", e))?;

        // 验证文件是否可以删除
        match fs::remove_file(updater_path) {
            Ok(_) => {
                fs::File::create(updater_path)
                    .map_err(|e| format!("重新创建文件失败: {}", e))?;
                Err("权限设置失败: 文件仍可被删除".to_string())
            },
            Err(_) => {
                Ok(())
            }
        }
    }

    /// macOS 平台禁用更新
    fn disable_macos_update(&self) -> Result<(), String> {
        // 处理 app-update.yml 文件
        let app_path = PathBuf::from("/Applications/Cursor.app/Contents/Resources/app-update.yml");
        if app_path.exists() {
            // 使用 osascript 获取 root 权限处理文件
            let script = format!(
                "do shell script \"mv '{}' '{}.backup' && touch '{}' && chmod a-w '{}'\" with prompt \"Cursor Agent 需要获取权限来修改文件\"",
                app_path.to_string_lossy(),
                app_path.to_string_lossy(),
                app_path.to_string_lossy(),
                app_path.to_string_lossy()
            );

            let output = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()
                .map_err(|e| format!("执行 osascript 失败: {}", e))?;

            if !output.status.success() {
                return Err(format!("处理配置文件失败: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }

        Ok(())
    }

    /// Linux 平台禁用更新
    fn disable_linux_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        // 执行命令序列
        Command::new("sh")
            .args([
                "-c",
                &format!(
                    "rm -rf '{}' && \
                     touch '{}' && \
                     chmod 444 '{}' && \
                     sudo chattr +i '{}'",
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy()
                ),
            ])
            .output()
            .map_err(|e| format!("执行命令失败: {}", e))?;

        Ok(())
    }

    /// 恢复 Cursor 自动更新
    pub fn restore_auto_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        match std::env::consts::OS {
            "windows" => self.restore_windows_update(updater_path),
            "macos" => self.restore_macos_update(),
            "linux" => self.restore_linux_update(updater_path),
            _ => Err("不支持的操作系统".to_string()),
        }
    }

    /// Windows 平台恢复更新
    fn restore_windows_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        // 删除现有的限制文件
        if updater_path.exists() {
            // 先移除文件属性
            Command::new("attrib")
                .args(["-r", "-s", "-h", updater_path.to_str().unwrap()])
                .output()
                .map_err(|e| format!("移除文件属性失败: {}", e))?;

            // 重置权限
            Command::new("icacls")
                .args([
                    updater_path.to_str().unwrap(),
                    "/reset"
                ])
                .output()
                .map_err(|e| format!("重置权限失败: {}", e))?;

            // 删除文件
            fs::remove_file(updater_path)
                .map_err(|e| format!("删除文件失败: {}", e))?;
        }

        // 检查是否存在备份
        let backup_path = updater_path.parent()
            .ok_or("无法获取父目录")?
            .join("cursor-updater.zip");

        if backup_path.exists() {
            // 解压备份
            let file = fs::File::open(&backup_path)
                .map_err(|e| format!("打开备份文件失败: {}", e))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("读取ZIP文件失败: {}", e))?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)
                    .map_err(|e| format!("访问ZIP文件条目失败: {}", e))?;
                let outpath = updater_path.join(file.name());

                if file.name().ends_with('/') {
                    fs::create_dir_all(&outpath)
                        .map_err(|e| format!("创建目录失败: {}", e))?;
                } else {
                    if let Some(p) = outpath.parent() {
                        fs::create_dir_all(p)
                            .map_err(|e| format!("创建父目录失败: {}", e))?;
                    }
                    let mut outfile = fs::File::create(&outpath)
                        .map_err(|e| format!("创建文件失败: {}", e))?;
                    std::io::copy(&mut file, &mut outfile)
                        .map_err(|e| format!("复制文件内容失败: {}", e))?;
                }
            }

            // 删除备份文件
            fs::remove_file(backup_path)
                .map_err(|e| format!("删除备份文件失败: {}", e))?;
        }

        Ok(())
    }

    /// macOS 平台恢复更新
    fn restore_macos_update(&self) -> Result<(), String> {
        // 恢复 app-update.yml 文件
        let app_path = PathBuf::from("/Applications/Cursor.app/Contents/Resources/app-update.yml");
        if app_path.exists() {
            // 使用 osascript 获取 root 权限处理文件
            let script = format!(
                "do shell script \"rm -f '{}' && mv '{}.backup' '{}'\" with prompt \"Cursor Agent 需要获取权限来修改文件\"",
                app_path.to_string_lossy(),
                app_path.to_string_lossy(),
                app_path.to_string_lossy()
            );

            let output = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()
                .map_err(|e| format!("执行 osascript 失败: {}", e))?;

            if !output.status.success() {
                return Err(format!("恢复配置文件失败: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }

        Ok(())
    }

    /// Linux 平台恢复更新
    fn restore_linux_update(&self, updater_path: &PathBuf) -> Result<(), String> {
        Command::new("sh")
            .args([
                "-c",
                &format!(
                    "sudo chattr -i '{}' 2>/dev/null; \
                     rm -f '{}' && \
                     if [ -f '{}.zip' ]; then \
                         unzip '{}.zip' -d '{}' && \
                         rm '{}.zip'; \
                     fi",
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy(),
                    updater_path.to_string_lossy(),
                    updater_path.parent().unwrap().to_string_lossy(),
                    updater_path.to_string_lossy()
                ),
            ])
            .output()
            .map_err(|e| format!("执行命令失败: {}", e))?;

        Ok(())
    }
}

impl Default for UpdateBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::paths::AppPaths;

    #[test]
    fn test_disable_update() {
        let paths = AppPaths::new().unwrap();
        let blocker = UpdateBlocker::new();
        
        match blocker.disable_auto_update(&paths.cursor_updater) {
            Ok(_) => println!("成功禁用自动更新"),
            Err(e) => println!("禁用自动更新失败: {}", e),
        }
    }

    #[test]
    fn test_restore_update() {
        let paths = AppPaths::new().unwrap();
        let blocker = UpdateBlocker::new();
        
        match blocker.restore_auto_update(&paths.cursor_updater) {
            Ok(_) => println!("成功恢复自动更新"),
            Err(e) => println!("恢复自动更新失败: {}", e),
        }
    }
}