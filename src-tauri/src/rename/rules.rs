use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RenameRules {
    pub prefix: String,
    pub suffix: String,
    pub find: String,
    pub replace: String,
    pub delete_chars: String,
    pub case_mode: String,
    pub numbering: bool,
    pub number_start: i64,
    pub number_step: i64,
    pub number_width: usize,
    pub number_position: String,
    pub number_separator: String,
}

impl Default for RenameRules {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            find: String::new(),
            replace: String::new(),
            delete_chars: String::new(),
            case_mode: "keep".into(),
            numbering: false,
            number_start: 1,
            number_step: 1,
            number_width: 3,
            number_position: "suffix".into(),
            number_separator: "_".into(),
        }
    }
}

impl RenameRules {
    pub fn validate(&self) -> Result<(), String> {
        if !["keep", "lower", "upper", "title"].contains(&self.case_mode.as_str()) {
            return Err("未知的大小写转换规则".into());
        }
        if self.numbering
            && (self.number_start < 0 || self.number_step < 1 || self.number_width > 12)
        {
            return Err("编号起始值需非负、步长需大于零，位数需介于 0～12".into());
        }
        if !["prefix", "suffix"].contains(&self.number_position.as_str()) {
            return Err("未知的编号位置".into());
        }
        Ok(())
    }

    pub fn apply(&self, name: &str, is_dir: bool, index: usize) -> Result<String, String> {
        let (stem, ext) = if !is_dir {
            match name.rfind('.') {
                Some(i) if i > 0 => (&name[..i], &name[i..]),
                _ => (name, ""),
            }
        } else {
            (name, "")
        };
        let mut base = if self.find.is_empty() {
            stem.to_owned()
        } else {
            stem.replace(&self.find, &self.replace)
        };
        base.retain(|c| !self.delete_chars.contains(c));
        base = match self.case_mode.as_str() {
            "lower" => base.to_lowercase(),
            "upper" => base.to_uppercase(),
            "title" => {
                let mut boundary = true;
                base.chars()
                    .flat_map(|c| {
                        let upper = boundary;
                        boundary = !c.is_alphanumeric();
                        if upper {
                            c.to_uppercase().collect::<Vec<_>>()
                        } else {
                            c.to_lowercase().collect()
                        }
                    })
                    .collect()
            }
            _ => base,
        };
        base = format!("{}{}{}", self.prefix, base, self.suffix);
        if self.numbering {
            let n = self
                .number_step
                .checked_mul(index as i64)
                .and_then(|n| self.number_start.checked_add(n))
                .ok_or("编号超出支持的范围")?;
            let number = format!("{:0width$}", n, width = self.number_width);
            base = if self.number_position == "prefix" {
                format!("{}{}{}", number, self.number_separator, base)
            } else {
                format!("{}{}{}", base, self.number_separator, number)
            };
        }
        Ok(format!("{base}{ext}"))
    }
}

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        return Err("名称不能为空或为 . / ..".into());
    }
    if name.ends_with([' ', '.']) {
        return Err("Windows 名称不能以空格或句点结尾".into());
    }
    if name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c)) {
        return Err("名称包含 Windows 非法字符".into());
    }
    if name.encode_utf16().count() > 255 {
        return Err("名称超过 255 个 UTF-16 字符".into());
    }
    let device = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&device.as_str())
        || ["COM", "LPT"].iter().any(|p| {
            device.strip_prefix(p).is_some_and(|n| {
                ["1", "2", "3", "4", "5", "6", "7", "8", "9", "¹", "²", "³"].contains(&n)
            })
        })
    {
        return Err("名称为 Windows 保留设备名".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rules_preserve_extension_and_do_not_recurse() {
        let r = RenameRules {
            prefix: "Paper_".into(),
            find: "IMG_".into(),
            replace: "".into(),
            numbering: true,
            number_position: "prefix".into(),
            ..Default::default()
        };
        assert_eq!(
            r.apply("IMG_001.jpg", false, 0).unwrap(),
            "001_Paper_001.jpg"
        );
        assert_eq!(
            r.apply("Folder.txt", true, 1).unwrap(),
            "002_Paper_Folder.txt"
        );
    }
    #[test]
    fn rejects_windows_invalid_names() {
        for n in [
            "",
            "..",
            "CON.txt",
            "com1",
            "LPT².bin",
            "a/b",
            "abc.",
            "abc ",
            "NUL",
        ] {
            assert!(validate_name(n).is_err(), "{n}");
        }
        for n in ["微信.txt", ".gitignore", "company", "file name.md"] {
            assert!(validate_name(n).is_ok());
        }
    }
}
