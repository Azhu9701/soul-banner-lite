use foundation::SoulProfile;

const CONTRADICTION_PATTERNS: &[(&str, &str)] = &[
    ("一方面", "另一方面"),
    ("虽然", "但是"),
    ("不可否认", "然而"),
    ("尽管", "但是"),
    ("诚然", "不过"),
];

const SHAKEN_MARKERS: &[&str] = &[
    "预设不成立",
    "前提假设有问题",
    "需要重新审视框架",
    "方法论局限",
    "我不确定",
    "可能有误",
    "需要验证",
];

const DOMAIN_KEYWORDS: &[(&str, &[&str])] = &[
    ("技术", &["代码", "算法", "系统", "架构", "性能"]),
    ("经济", &["市场", "价格", "成本", "利润", "投资"]),
    ("政治", &["政策", "政府", "权力", "选举", "制度"]),
    ("哲学", &["存在", "意识", "真理", "价值", "逻辑"]),
];

#[derive(Debug, Clone)]
pub struct AuditResult {
    pub passed: bool,
    pub contradictions: Vec<String>,
    pub blind_spot_alerts: Vec<String>,
    pub premise_shaken: Vec<String>,
    pub revision_needed: bool,
    pub suggested_proposals: Vec<RevisionProposalSuggestion>,
}

#[derive(Debug, Clone)]
pub struct RevisionProposalSuggestion {
    pub proposal_type: String,
    pub title: String,
    pub description: String,
    pub suggested_changes: String,
}

impl AuditResult {
    pub fn clean() -> Self {
        AuditResult {
            passed: true,
            contradictions: vec![],
            blind_spot_alerts: vec![],
            premise_shaken: vec![],
            revision_needed: false,
            suggested_proposals: vec![],
        }
    }

    pub fn has_issues(&self) -> bool {
        !self.contradictions.is_empty() || 
        !self.blind_spot_alerts.is_empty() || 
        !self.premise_shaken.is_empty() ||
        self.revision_needed
    }
}

pub struct SelfAudit;

impl SelfAudit {
    /// 核心审计逻辑（纯文本分析，无副作用）
    pub fn audit(profile: &SoulProfile, task: &str, output: &str) -> AuditResult {
        let mut result = AuditResult::clean();

        Self::check_excluded_scenarios(profile, task, output, &mut result);
        Self::check_contradictions(profile, output, &mut result);
        Self::check_boundary_violations(profile, output, &mut result);
        Self::check_premise_shaken(profile, output, &mut result);
        Self::check_domain_completeness(profile, task, output, &mut result);

        let suggestions = Self::generate_suggestions(&result, profile);
        result.suggested_proposals = suggestions;

        result
    }

    fn check_excluded_scenarios(
        profile: &SoulProfile,
        task: &str,
        output: &str,
        result: &mut AuditResult,
    ) {
        for scenario in &profile.exclude_scenarios {
            if task.contains(scenario) || output.contains(scenario) {
                result.blind_spot_alerts.push(format!(
                    "触碰排除场景「{}」— 魂 {} 声明不适用于此类场景",
                    scenario, profile.name
                ));
                result.passed = false;
                result.revision_needed = true;
            }
        }
    }

    fn check_contradictions(
        _profile: &SoulProfile,
        output: &str,
        result: &mut AuditResult,
    ) {
        let pairs_found = CONTRADICTION_PATTERNS
            .iter()
            .filter(|(a, b)| output.contains(a) && output.contains(b))
            .count() as u32;
        if pairs_found >= 4 {
            result.contradictions.push(format!(
                "输出含 {} 组转折结构——可能是辩证思考，也可能是立场不明确。请人工判断",
                pairs_found
            ));
            // 不再自动触发 revision_needed——辩证表达是合理的，交给人工判断
        }
    }

    fn check_boundary_violations(
        profile: &SoulProfile,
        output: &str,
        result: &mut AuditResult,
    ) {
        if !profile.self_declare.is_empty() {
            let boundary_violations = profile
                .self_declare
                .split(|c: char| c == '，' || c == '、' || c == '。' || c == '\n')
                .map(|token| {
                    let t = token.trim();
                    if t.is_empty() {
                        return 0u32;
                    }
                    let negated = ["不是", t].concat();
                    if output.contains(&negated) && !output.contains(t) {
                        1
                    } else {
                        0
                    }
                })
                .sum::<u32>();
            if boundary_violations > 0 {
                result.contradictions.push(format!(
                    "输出含 {} 处可能超出 self_declare 边界",
                    boundary_violations
                ));
                result.revision_needed = true;
            }
        }
    }

    fn check_premise_shaken(
        profile: &SoulProfile,
        output: &str,
        result: &mut AuditResult,
    ) {
        for marker in SHAKEN_MARKERS {
            if output.contains(marker) {
                result.premise_shaken.push(format!(
                    "魂 {} 输出标记前提动摇：「{}」出现在分析中",
                    profile.name, marker
                ));
                result.revision_needed = true;
            }
        }
    }

    fn check_domain_completeness(
        profile: &SoulProfile,
        task: &str,
        output: &str,
        result: &mut AuditResult,
    ) {
        for (domain, keywords) in DOMAIN_KEYWORDS {
            let task_has_domain = keywords.iter().any(|&k| task.contains(k));
            let output_has_domain = keywords.iter().any(|&k| output.contains(k));
            
            if task_has_domain && !output_has_domain && !profile.domains.contains(&domain.to_string()) {
                result.blind_spot_alerts.push(format!(
                    "任务涉及「{}」领域，但魂 {} 未声明该领域且输出未覆盖相关关键词",
                    domain, profile.name
                ));
                result.revision_needed = true;
            }
        }
    }

    fn generate_suggestions(
        audit_result: &AuditResult,
        profile: &SoulProfile,
    ) -> Vec<RevisionProposalSuggestion> {
        let mut suggestions = Vec::new();
        
        if !audit_result.blind_spot_alerts.is_empty() {
            suggestions.push(RevisionProposalSuggestion {
                proposal_type: "BlindSpotMitigation".to_string(),
                title: format!("补充 {} 的盲区覆盖", profile.name),
                description: "审计发现魂在某些领域的知识存在空白".to_string(),
                suggested_changes: "建议更新 exclude_scenarios 或补充相关领域训练".to_string(),
            });
        }

        if !audit_result.contradictions.is_empty() {
            suggestions.push(RevisionProposalSuggestion {
                proposal_type: "BoundaryAdjustment".to_string(),
                title: format!("调整 {} 的声明边界", profile.name),
                description: "审计发现魂的输出可能存在与声明不一致的情况".to_string(),
                suggested_changes: "建议审查并更新 self_declare".to_string(),
            });
        }

        if !audit_result.premise_shaken.is_empty() {
            suggestions.push(RevisionProposalSuggestion {
                proposal_type: "OntologyUpdate".to_string(),
                title: format!("更新 {} 的本体论框架", profile.name),
                description: "审计发现魂对前提的稳定性产生了疑问".to_string(),
                suggested_changes: "建议审查并优化 ismism_code 和基础框架".to_string(),
            });
        }
        
        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use foundation::EffectivenessStats;

    fn create_test_profile() -> SoulProfile {
        SoulProfile {
            name: "TestSoul".to_string(),
            ismism_code: "0-0-0-0".to_string(),
            field: "Test".to_string(),
            ontology: "".to_string(),
            epistemology: "".to_string(),
            teleology: "".to_string(),
            domains: vec!["技术".to_string()],
            exclude_scenarios: vec!["军事".to_string(), "医疗".to_string()],
            summon_count: 0,
            effectiveness: EffectivenessStats::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
            summon_prompt: "你是一个测试魂".to_string(),
            practice_observations: vec![],
            title: "".to_string(),
            description: "".to_string(),
            voice: "".to_string(),
            mind: "".to_string(),
            self_declare: "我只讨论技术问题".to_string(),
            skills_expertise: vec![],
            model: "".to_string(),
            tools: "".to_string(),
            trigger_keywords: vec![],
            compat: vec![],
            incompat: vec![],
        }
    }

    #[test]
    fn test_audit_clean() {
        let result = AuditResult::clean();
        assert!(result.passed);
        assert!(!result.revision_needed);
    }

    #[test]
    fn test_audit_logic() {
        let profile = create_test_profile();
        let mut result = AuditResult::clean();
        
        for scenario in &profile.exclude_scenarios {
            if "讨论军事策略".contains(scenario) {
                result.blind_spot_alerts.push(format!(
                    "触碰排除场景「{}」— 魂 {} 声明不适用于此类场景",
                    scenario, profile.name
                ));
                result.passed = false;
                result.revision_needed = true;
            }
        }
        
        assert_eq!(result.blind_spot_alerts.len(), 1);
        assert!(result.revision_needed);
    }

    #[test]
    fn test_contradiction_detection() {
        let _profile = create_test_profile();
        
        let pairs_found = CONTRADICTION_PATTERNS
            .iter()
            .filter(|(a, b)| {
                "一方面这很好，但是另一方面有问题。虽然可行，不过需要谨慎。".contains(a) && 
                "一方面这很好，但是另一方面有问题。虽然可行，不过需要谨慎。".contains(b)
            })
            .count() as u32;
        
        assert!(pairs_found >= 2);
    }
}