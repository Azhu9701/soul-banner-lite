use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use foundation::{LLMRequest, CallConfig, Prompt, PromptMessage, Provider, SoulListEntry, SoulProfile};
use possession::PossessionInput;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::error::{map_api_error, ApiError};
use crate::ocr;
use crate::state::{AppState, InterrogationGate, InterrogationQuestion};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(start_possession))
        .route("/court", post(start_court_session))
        .route("/business", post(start_business_session))
        .route("/analyze", post(analyze_task))
        .route("/ocr", post(ocr_upload))
        .route("/interrogate", post(start_interrogation))
        .route("/interrogate/:gate_id/respond", post(submit_interrogation))
        .route("/:session_id/status", get(possession_status))
        .route("/:session_id/follow-up", post(follow_up))
}

#[derive(Debug, Deserialize)]
struct StartPossessionRequest {
    #[serde(default)] mode: Option<String>,
    task: String,
    #[serde(default)] souls: Vec<String>,
    #[serde(default)] topic: Option<String>,
    #[serde(default)] judgment: Option<String>,
    #[serde(default)] worry: Option<String>,
    #[serde(default)] unknown: Option<String>,
    #[serde(default)] interrogation_context: Option<String>,
    #[serde(default)] task_cards: std::collections::HashMap<String, String>,
    #[serde(default)] search_topic: bool,
}

#[derive(Debug, Serialize)]
struct StartPossessionResponse { session_id: String, mode: String, ws_url: String }

async fn start_possession(
    State(state): State<Arc<AppState>>,
    Json(body): Json<StartPossessionRequest>,
) -> Result<Json<StartPossessionResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    if body.task.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(ApiError { error: "task is required".into() })));
    }
    tracing::info!("Starting possession with {} souls, search_topic={}", body.souls.len(), body.search_topic);
    
    // 议题搜索：在召唤魂之前，通过 SearXNG 实时搜索 Web 获取背景信息
    let search_results = if body.search_topic {
        let trunc_len = body.task.len().min(80);
        let safe_idx = (0..=trunc_len).rev().find(|&i| body.task.is_char_boundary(i)).unwrap_or(0);
        tracing::info!("Running SearXNG topic search for: {}", &body.task[..safe_idx]);
        match state.collector.search_topic_searxng(&body.task, 3).await {
            Ok(md) => {
                tracing::info!("SearXNG topic search returned {} bytes", md.len());
                Some(md)
            }
            Err(e) => {
                tracing::warn!("SearXNG topic search failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    let input = PossessionInput {
        mode: body.mode.and_then(|m| foundation::PossessionMode::from_str(&m)),
        task: body.task, souls: body.souls, topic: body.topic,
        judgment: body.judgment, worry: body.worry, unknown: body.unknown,
        interrogation_context: body.interrogation_context,
        task_cards: body.task_cards,
        search_topic: body.search_topic,
        search_results,
    };
    let (tx, rx) = tokio::sync::mpsc::channel::<possession::WsEvent>(256);
    let session_id = state.engine.start_possession(input, tx).await.map_err(map_api_error)?;
    tracing::info!("Possession session created: {}", session_id);

    // Relay: forward events from dispatch_mode to WebSocket subscribers
    let ws = state.engine.ws_manager().clone();
    let sid = session_id.clone();
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(event) = rx.recv().await {
            ws.broadcast_system(&sid, &event);
        }
        // Don't remove session - keep it for follow-up questions
    });

    Ok(Json(StartPossessionResponse {
        ws_url: format!("/ws/possess/{}/main", session_id), session_id, mode: "unknown".into(),
    }))
}

// ── 模拟仲裁庭快捷入口 ──

#[derive(Debug, Deserialize)]
struct CourtSessionRequest {
    task: String,
    #[serde(default)]
    judgment: Option<String>,
    #[serde(default)]
    worry: Option<String>,
    #[serde(default)]
    unknown: Option<String>,
}

/// POST /possess/court — 一键启动模拟仲裁庭
/// 固定 5 角色（仲裁法官/原告律师/被告律师/专家证人/劳动者之声）+ 动态 task_cards
async fn start_court_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CourtSessionRequest>,
) -> Result<Json<StartPossessionResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    if body.task.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(ApiError { error: "案件描述不能为空".into() })));
    }

    let court_souls = vec![
        "仲裁法官".to_string(),
        "原告律师".to_string(),
        "被告律师".to_string(),
        "专家证人".to_string(),
        "劳动者之声".to_string(),
    ];

    let case_type = detect_case_type(&body.task);
    tracing::info!("Court case type detected: {:?}", case_type);
    let task_cards = build_dynamic_task_cards(case_type);

    tracing::info!("Starting court session: 5 roles");

    let input = PossessionInput {
        mode: Some(foundation::PossessionMode::Conference),
        task: body.task,
        souls: court_souls,
        topic: None,
        judgment: body.judgment,
        worry: body.worry,
        unknown: body.unknown,
        interrogation_context: None,
        task_cards,
        search_topic: false,
        search_results: None,
    };

    let (tx, rx) = tokio::sync::mpsc::channel::<possession::WsEvent>(256);
    let session_id = state.engine.start_possession(input, tx).await.map_err(map_api_error)?;
    tracing::info!("Court session created: {}", session_id);

    let ws = state.engine.ws_manager().clone();
    let sid = session_id.clone();
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(event) = rx.recv().await {
            ws.broadcast_system(&sid, &event);
        }
    });

    Ok(Json(StartPossessionResponse {
        ws_url: format!("/ws/possess/{}/main", session_id), session_id, mode: "conference".into(),
    }))
}

/// 根据案件描述中的关键词检测案件类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum CaseType {
    WrongfulDismissal,  // 违法解除/辞退/开除
    WageArrears,        // 欠薪/工资纠纷
    WorkInjury,         // 工伤/职业病
    Discrimination,     // 歧视/骚扰
    ContractDispute,    // 合同纠纷/竞业限制
    General,            // 通用
}

fn detect_case_type(task: &str) -> CaseType {
    let t = task;
    // 优先精确匹配，再回退到通用匹配。
    // 注意：劳动者常同时提及多个问题（如"欠薪后开除"），此函数按首次命中返回。
    // 多问题复杂案件会归入第一个匹配的类型——覆盖主要诉求即可。
    if t.contains("开除") || t.contains("辞退") || t.contains("解雇") 
       || (t.contains("解除") && !t.contains("竞业限制")) {
        CaseType::WrongfulDismissal
    } else if t.contains("工资") || t.contains("欠薪") || t.contains("拖欠") || t.contains("加班费") || t.contains("克扣") {
        CaseType::WageArrears
    } else if t.contains("工伤") || t.contains("职业病") || t.contains("伤残") || t.contains("猝死") || t.contains("安全事故") {
        CaseType::WorkInjury
    } else if t.contains("歧视") || t.contains("骚扰") || t.contains("平等") || t.contains("性别") || t.contains("怀孕") {
        CaseType::Discrimination
    } else if t.contains("竞业限制") || t.contains("保密") || t.contains("服务期") || t.contains("培训费") || t.contains("违约金") {
        CaseType::ContractDispute
    } else {
        CaseType::General
    }
}

fn build_dynamic_task_cards(case_type: CaseType) -> std::collections::HashMap<String, String> {
    let mut cards = std::collections::HashMap::new();

    // 法官
    let judge_card = match case_type {
        CaseType::WrongfulDismissal => 
            "你是本案仲裁法官，正在主持庭审。本案是违法解除纠纷。你重点审查：1. 解除的理由是什么？是否有书面通知？2. 解除程序是否合法（是否通知工会？是否书面送达？）3. 举证责任在用人单位——让被告举证解除合法。你的任务：归纳争议焦点、确认无异议事实、追问双方、用 search_labor_law 检索相关法条、用 calculate_severance 独立核算。注意：庭审阶段不做出最终裁决——裁决留在退庭评议后进行。",
        CaseType::WageArrears => 
            "你是本案仲裁法官，正在主持庭审。本案是工资/报酬纠纷。你重点审查：1. 工资标准是什么（合同约定？实际发放？）2. 欠薪金额和期间？3. 劳动者是否已催告？你的任务：归纳争议焦点、确认无异议事实、追问双方、用 search_labor_law 检索法条、用 calculate_severance 独立核算。注意：庭审阶段不做出最终裁决。",
        CaseType::WorkInjury => 
            "你是本案仲裁法官，正在主持庭审。本案是工伤/职业病纠纷。你重点审查：1. 是否已进行工伤认定？认定结论是什么？2. 伤残等级鉴定结果？3. 用人单位是否缴纳工伤保险？4. 医疗费用和停工留薪期。你的任务：归纳争议焦点、确认无异议事实、追问双方、用 search_labor_law 检索相关法条。注意：庭审阶段不做出最终裁决。",
        CaseType::Discrimination => 
            "你是本案仲裁法官，正在主持庭审。本案涉及就业歧视/骚扰。你重点审查：1. 是否存在差别对待的直接证据？2. 差别对待是否具有合法理由？3. 举证责任分配——劳动者初步举证后由用人单位说明正当理由。你的任务：归纳争议焦点、确认无异议事实、追问双方、用 search_labor_law 检索反歧视相关法条。注意：庭审阶段不做出最终裁决。",
        CaseType::ContractDispute => 
            "你是本案仲裁法官，正在主持庭审。本案是合同条款纠纷。你重点审查：1. 争议条款的具体内容是什么？2. 条款是否合法有效？3. 违约金是否过高？4. 竞业限制是否支付了补偿？你的任务：归纳争议焦点、确认无异议事实、追问双方、用 search_labor_law 检索相关法条。注意：庭审阶段不做出最终裁决。",
        CaseType::General => 
            "你是本案仲裁法官，正在主持庭审。你的任务是：1. 归纳本案争议焦点并当庭说明 2. 确认双方无异议的事实 3. 指明举证责任分配 4. 对双方律师的陈述提出追问 5. 用 search_labor_law 检索相关法条、用 calculate_severance 独立核算。注意：庭审阶段不做出最终裁决。",
    };
    cards.insert("仲裁法官".to_string(), judge_card.to_string());

    // 原告律师
    let plaintiff_card = match case_type {
        CaseType::WrongfulDismissal =>
            "你代理劳动者。本案是违法解除纠纷。请：1. 陈述解除的事实经过（什么时候、谁通知的、什么理由、有没有书面文件）2. 用 search_labor_law 检索违法解除的法律依据 3. 用 calculate_severance 算 2N 赔偿金 4. 用 generate_evidence_checklist 生成证据清单（重点：解除通知书、工资流水、劳动合同）5. 预判被告可能的抗辩并提前反驳。",
        CaseType::WageArrears =>
            "你代理劳动者。本案是欠薪纠纷。请：1. 列出欠薪明细（哪几个月、每个月多少、总计多少）2. 用 search_labor_law 检索工资支付相关法条 3. 用 calculate_severance 算欠薪+额外补偿 4. 用 generate_evidence_checklist 生成证据清单（重点：工资条、银行流水、考勤记录、催告记录）5. 预判被告可能的抗辩。",
        CaseType::WorkInjury =>
            "你代理劳动者。本案是工伤纠纷。请：1. 陈述工伤发生经过（时间、地点、原因）2. 用 search_labor_law 检索工伤保险条例相关条款 3. 列出各项赔偿项目（医疗费/停工留薪/伤残补助等）并估算金额 4. 用 generate_evidence_checklist 生成证据清单（重点：工伤认定书、医疗记录、工资证明）5. 如果单位未缴工伤保险，主张由单位全额承担。",
        CaseType::Discrimination =>
            "你代理劳动者。本案是歧视/骚扰纠纷。请：1. 陈述歧视或骚扰的具体事实（什么行为、发生时间、频率、有无证人）2. 用 search_labor_law 检索平等就业相关法条 3. 列出可主张的赔偿项目（精神损害/工资损失等）4. 用 generate_evidence_checklist 生成证据清单（重点：聊天记录、邮件、录音、证人、投诉记录）5. 引用类案判例支持你的主张。",
        CaseType::ContractDispute =>
            "你代理劳动者。本案是合同条款纠纷。请：1. 指出争议条款的具体内容 2. 用 search_labor_law 检索条款是否有效（如竞业限制未付补偿=无效）3. 如果涉及违约金，分析是否过高（超过培训费用=不合法）4. 用 calculate_severance 计算如果解除合同应得的补偿 5. 用 generate_evidence_checklist 生成证据清单。",
        CaseType::General =>
            "你代理劳动者。请：1. 陈述劳动者的诉讼请求和事实依据 2. 列举支持主张的证据 3. 用 calculate_severance 计算应得补偿金额 4. 用 generate_evidence_checklist 生成证据清单 5. 反驳被告可能的抗辩理由。",
    };
    cards.insert("原告律师".to_string(), plaintiff_card.to_string());

    // 被告律师
    let defendant_card = match case_type {
        CaseType::WrongfulDismissal =>
            "你代理用人单位。本案是违法解除纠纷。请：1. 说明解除的具体原因和程序（试用期不合格？严重违纪？不能胜任？）2. 用 search_labor_law 检索合法解除的法律依据 3. 用 calculate_severance 独立核算——如果确实违法，实事求是不狡辩 4. 如果解除合法，用 generate_evidence_checklist 梳理你方证据（培训记录、违纪证据、考核结果）5. 逐条回应原告主张。",
        CaseType::WageArrears =>
            "你代理用人单位。本案是欠薪纠纷。请：1. 说明工资结构和支付记录——哪些已付、哪些有争议 2. 用 search_labor_law 检索工资支付相关法条 3. 用 calculate_severance 独立核算应发金额 4. 对原告列出的欠薪明细逐条回应 5. 如果确实欠薪，解释原因并提出支付方案。",
        CaseType::WorkInjury =>
            "你代理用人单位。本案是工伤纠纷。请：1. 说明是否已为劳动者缴纳工伤保险 2. 用 search_labor_law 检索用人单位的法定义务 3. 如果已认定工伤，逐项回应赔偿诉求——哪些应由社保基金支付、哪些由单位承担 4. 用 generate_evidence_checklist 梳理你方证据（社保缴纳记录、安全培训记录、事故报告）5. 对不合理的索赔项目提出异议。",
        CaseType::Discrimination =>
            "你代理用人单位。本案是歧视/骚扰纠纷。请：1. 说明涉事行为的处理情况（是否已调查、是否已处分相关人员）2. 用 search_labor_law 检索用人单位的反歧视义务 3. 逐条回应原告指控——哪些不属实、哪些已处理 4. 用 generate_evidence_checklist 梳理你方证据（规章制度、调查记录、处理决定）5. 如果存在管理疏漏，诚实承认并提出改进措施。",
        CaseType::ContractDispute =>
            "你代理用人单位。本案是合同条款纠纷。请：1. 说明争议条款的设立目的和合理性 2. 用 search_labor_law 检索条款的合法性依据 3. 如果涉及竞业限制，说明是否已支付补偿 4. 用 calculate_severance 核算如果条款无效的财务影响 5. 用 generate_evidence_checklist 梳理你方证据（培训费用凭证、保密协议、补偿支付记录）。",
        CaseType::General =>
            "你代理用人单位。请：1. 陈述答辩意见 2. 对原告主张逐项回应（认可/否认/不知情）3. 提出抗辩理由和反证 4. 用 calculate_severance 独立核算补偿金额 5. 用 generate_evidence_checklist 梳理你方证据 6. 用 search_labor_law 检索对你方有利的法条和判例。",
    };
    cards.insert("被告律师".to_string(), defendant_card.to_string());

    // 专家证人
    let expert_card = match case_type {
        CaseType::WrongfulDismissal =>
            "你是中立的行业专家。本案是违法解除纠纷。请：1. 从行业惯例角度评估——此类解除在同行业中的普遍处理方式 2. '不能胜任'的认定标准在同行业中是什么 3. 合规的解除程序应包括哪些步骤 4. 不偏袒任何一方——如果行业惯例支持某一方，如实说。",
        CaseType::WageArrears =>
            "你是中立的行业专家。本案是欠薪纠纷。请：1. 同行业同岗位的薪酬水平是多少 2. 工资结构中基本工资与绩效/奖金的常见比例 3. 绩效工资的考核标准在同行业中如何设置 4. 欠薪对劳动者生计的实际影响。",
        CaseType::WorkInjury =>
            "你是中立的行业专家。本案是工伤纠纷。请：1. 该行业常见的安全风险和工伤类型 2. 行业通行的安全防护标准和培训要求 3. 同类岗位的工伤发生率和赔偿标准 4. 如果用人单位的安全措施低于行业标准，直接指出。",
        CaseType::Discrimination =>
            "你是中立的行业专家。本案是歧视/骚扰纠纷。请：1. 行业内招聘和晋升的常见做法 2. 同类企业在反歧视/反骚扰方面的制度建设情况 3. 差别对待是否有业务合理性的可能性 4. 不回避行业中的问题——如果该行业确实存在普遍性歧视，如实说。",
        CaseType::ContractDispute =>
            "你是中立的行业专家。本案是合同条款纠纷。请：1. 竞业限制/保密/培训服务期条款在同行业中的普遍做法 2. 竞业限制补偿金的市场通行标准 3. 培训费用的合理范围和计算方式 4. 条款是否在合理范围内——过高或过低都要指出。",
        CaseType::General =>
            "你是中立的行业专家。请：1. 就案件涉及的专业问题给出意见 2. 说明行业通行做法 3. 评估双方主张的合理性 4. 不偏袒任何一方。",
    };
    cards.insert("专家证人".to_string(), expert_card.to_string());

    // 劳动者（当事人）
    let worker_card = match case_type {
        CaseType::WrongfulDismissal =>
            "你是当事人本人。请用第一人称陈述：1. 什么时候入职、做什么工作、月薪多少 2. 什么时间被通知解除、谁通知的、给的理由是什么、有没有书面文件 3. 你觉得真正的原因是什么（和通知的理由一样吗？）4. 用 search_labor_law 查你的权利——这样解除合不合法 5. 用 calculate_severance 算你应该拿多少赔偿 6. 你有什么证据。",
        CaseType::WageArrears =>
            "你是当事人本人。请用第一人称陈述：1. 什么时间入职、做什么工作、合同约定的工资是多少 2. 从什么时候开始欠薪、每个月欠多少、总共欠多少 3. 你有没有催过公司——什么时候、通过什么方式、公司怎么回复的 4. 用 search_labor_law 查你的权利 5. 用 calculate_severance 算公司应该补你多少钱 6. 你有什么证据（工资条、银行流水、聊天记录）。",
        CaseType::WorkInjury =>
            "你是当事人本人。请用第一人称陈述：1. 什么时间、在什么地点、做什么工作时受的伤 2. 受伤后谁送你去医院的、公司有没有给你申请工伤认定 3. 你现在的伤情——医疗花了多少钱、休息了多久、有没有后遗症 4. 用 search_labor_law 查工伤赔偿的权利 5. 用 calculate_severance 估算各项赔偿 6. 你有什么证据（医疗记录、工伤认定书、工资证明）。",
        CaseType::Discrimination =>
            "你是当事人本人。请用第一人称陈述：1. 你遭遇了什么歧视或骚扰——具体行为、发生时间、频率 2. 有没有向上级或HR反映过——结果是什么 3. 这对你的工作和生活造成了什么影响 4. 用 search_labor_law 查平等就业的权利 5. 你有什么证据（聊天记录、邮件、录音、证人名字）6. 你想要什么结果。",
        CaseType::ContractDispute =>
            "你是当事人本人。请用第一人称陈述：1. 争议条款是什么——什么时候签的、你当时理解这条款吗 2. 这条款对你造成了什么限制（不能去什么公司？要赔多少钱？）3. 公司有没有给你相应的补偿（如竞业限制补偿金）4. 用 search_labor_law 查合同条款是否合法 5. 用 calculate_severance 算如果解除合同该拿多少钱 6. 你有什么证据。",
        CaseType::General =>
            "你是当事人本人。请用第一人称陈述：1. 到底发生了什么（按时间线）2. 你的感受和处境 3. 用 search_labor_law 查你的权利 4. 用 calculate_severance 算你该拿多少钱 5. 你有哪些证据 6. 你想要什么结果。",
    };
    cards.insert("劳动者之声".to_string(), worker_card.to_string());

    cards
}

// ── 商业推演快捷入口 ──

#[derive(Debug, Deserialize)]
struct BusinessSessionRequest {
    task: String,
    #[serde(default)]
    judgment: Option<String>,
    #[serde(default)]
    worry: Option<String>,
    #[serde(default)]
    unknown: Option<String>,
}

/// POST /possess/business — 一键启动商业沙盘推演
/// 固定 5 角色（CEO/CFO/COO/CMO/管理咨询顾问）+ 动态 task_cards
async fn start_business_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BusinessSessionRequest>,
) -> Result<Json<StartPossessionResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    if body.task.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, Json(ApiError { error: "经营议题不能为空".into() })));
    }

    let business_souls = vec![
        "CEO".to_string(),
        "CFO".to_string(),
        "COO".to_string(),
        "CMO".to_string(),
        "管理咨询顾问".to_string(),
    ];

    let scenario = detect_business_scenario(&body.task);
    tracing::info!("Business scenario detected: {:?}", scenario);
    let task_cards = build_business_task_cards(scenario);

    tracing::info!("Starting business session: 5 roles");

    let input = PossessionInput {
        mode: Some(foundation::PossessionMode::Conference),
        task: body.task,
        souls: business_souls,
        topic: None,
        judgment: body.judgment,
        worry: body.worry,
        unknown: body.unknown,
        interrogation_context: None,
        task_cards,
        search_topic: false,
        search_results: None,
    };

    let (tx, rx) = tokio::sync::mpsc::channel::<possession::WsEvent>(256);
    let session_id = state.engine.start_possession(input, tx).await.map_err(map_api_error)?;
    tracing::info!("Business session created: {}", session_id);

    let ws = state.engine.ws_manager().clone();
    let sid = session_id.clone();
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(event) = rx.recv().await {
            ws.broadcast_system(&sid, &event);
        }
    });

    Ok(Json(StartPossessionResponse {
        ws_url: format!("/ws/possess/{}/main", session_id), session_id, mode: "conference".into(),
    }))
}

/// 根据经营议题检测商业推演场景类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum BusinessScenario {
    StrategicPlanning,   // 战略规划/市场扩张
    FinancialDecision,   // 融资/投资/财务决策
    OperationOptimization, // 运营优化/成本控制
    ProductLaunch,       // 新品上市/产品规划
    CrisisManagement,    // 危机管理/现金流断裂
    GeneralBusiness,     // 通用
}

fn detect_business_scenario(task: &str) -> BusinessScenario {
    let t = task;
    if t.contains("战略") || t.contains("扩张") || t.contains("新市场") || t.contains("增长") || t.contains("竞争") || t.contains("定位") {
        BusinessScenario::StrategicPlanning
    } else if t.contains("融资") || t.contains("投资") || t.contains("贷款") || t.contains("上市") || t.contains("资本") || t.contains("估值") || t.contains("股东") {
        BusinessScenario::FinancialDecision
    } else if t.contains("成本") || t.contains("效率") || t.contains("供应链") || t.contains("库存") || t.contains("产能") || t.contains("降本") {
        BusinessScenario::OperationOptimization
    } else if t.contains("新品") || t.contains("产品") || t.contains("研发") || t.contains("上市") || t.contains("技术") || t.contains("创新") {
        BusinessScenario::ProductLaunch
    } else if t.contains("危机") || t.contains("亏损") || t.contains("破产") || t.contains("现金流断裂") || t.contains("崩塌") || t.contains("拯救") {
        BusinessScenario::CrisisManagement
    } else {
        BusinessScenario::GeneralBusiness
    }
}

fn build_business_task_cards(scenario: BusinessScenario) -> std::collections::HashMap<String, String> {
    let mut cards = std::collections::HashMap::new();

    // CEO 任务卡
    let ceo_card = match scenario {
        BusinessScenario::StrategicPlanning =>
            "你是公司的CEO。本次推演议题是战略规划/市场扩张。请：1. 明确公司的战略定位和核心竞争力 2. 设定中长期经营目标（市场地位、营收规模）和年度目标 3. 提出具体的战略路径——是成本领先、差异化还是聚焦？4. 评估进入新市场的优先级和节奏 5. 明确资源分配的总体原则。你的决策将影响其他职能部门的计划。",
        BusinessScenario::FinancialDecision =>
            "你是公司的CEO。本次推演议题是融资/投资决策。请：1. 明确公司的资本战略——为什么需要融资/为什么要投资 2. 评估不同融资方案对公司控制权和经营自主权的影响 3. 设定财务目标——期望的资本回报率和风险容忍度 4. 平衡短期资金需求和长期战略布局 5. 做出最终决策并说明理由。",
        BusinessScenario::OperationOptimization =>
            "你是公司的CEO。本次推演议题是运营优化/成本控制。请：1. 明确运营优化的战略目标——是追求利润率提升还是规模扩张 2. 评估不同优化方案对客户体验和员工士气的潜在影响 3. 设定成本降低的目标和底线（不能低于什么标准）4. 在运营效率和长期投入之间做出决断 5. 明确各职能部门的优先级。",
        BusinessScenario::ProductLaunch =>
            "你是公司的CEO。本次推演议题是新产品上市/产品规划。请：1. 明确新产品的战略定位——是市场领先还是市场跟随？2. 设定新产品的市场目标和时间表 3. 评估不同产品路线图的资源需求和风险 4. 决定产品开发和上市的资源投入力度 5. 平衡现有产品线和创新产品的资源配置。",
        BusinessScenario::CrisisManagement =>
            "你是公司的CEO。本次推演议题是危机管理/企业经营困境。请：1. 坦诚评估企业当前的危机严重程度 2. 提出紧急应对方案——止血、保现金流、稳定核心团队 3. 设定分阶段的复苏目标 4. 评估不同方案的风险和代价 5. 做出果断决策——这是一个需要领导力的时刻。",
        BusinessScenario::GeneralBusiness =>
            "你是公司的CEO。请从全局战略角度分析当前的经营议题：1. 明确核心问题和关键决策点 2. 设定清晰的目标和衡量标准 3. 提出战略方向和资源分配原则 4. 听取各职能部门的分析后做出最终决策。",
    };
    cards.insert("CEO".to_string(), ceo_card.to_string());

    // CFO 任务卡
    let cfo_card = match scenario {
        BusinessScenario::StrategicPlanning =>
            "你是公司的CFO。本次推演议题是战略规划/市场扩张。请：1. 测算扩张方案的资金需求（市场开发费用、产能建设投资、运营资金）2. 分析不同扩张节奏对现金流的影响 3. 评估融资方案——内源性融资 vs 外部融资的配比 4. 用财务指标（ROE、EVA）评估战略方案的可行性 5. 明确财务风险底线——不能突破的负债率和现金流安全线。",
        BusinessScenario::FinancialDecision =>
            "你是公司的CFO。本次推演议题是融资/投资决策。请：1. 分析当前资本结构（资产负债率、长短期负债配比）2. 测算不同融资方案的资金成本和财务风险 3. 用EVA评估投资方案的可行性——税后净营业利润-资本成本 4. 用杜邦公式分析不同方案对ROE的影响 5. 关注现金流预测——确保公司在任何时点不会断裂。",
        BusinessScenario::OperationOptimization =>
            "你是公司的CFO。本次推演议题是运营优化/成本控制。请：1. 用ABC成本法分析各产品的真实盈利能力——直接成本+间接成本分摊 2. 识别费用结构中的不合理支出 3. 测算不同优化方案的财务影响 4. 建议需要削减的费用项和保留的投资项 5. 设定明确的财务指标目标。",
        BusinessScenario::ProductLaunch =>
            "你是公司的CFO。本次推演议题是新产品上市/产品规划。请：1. 计算新产品的全周期投资回报——研发费用、产能建设、市场推广、预期收益 2. 评估不同产品路线图的资金需求和时间安排 3. 分析各产品的毛利率和净利率 4. 用波士顿矩阵辅助分析产品组合的财务健康度 5. 提出财务维度的优先级建议。",
        BusinessScenario::CrisisManagement =>
            "你是公司的CFO。本次推演议题是危机管理/企业经营困境。请：1. 立即评估现金流状况——可用现金、未来3个月的现金流入/流出 2. 列出紧急融资方案——应收账款贴现、短期贷款、出售资产 3. 测算各项费用的削减空间 4. 分析最坏情况下的生存期限（runway）5. 提出财务维度的紧急行动方案。",
        BusinessScenario::GeneralBusiness =>
            "你是公司的CFO。请从财务角度分析当前经营议题：1. 分析经营决策对三张报表的影响 2. 评估现金流风险和融资需求 3. 计算关键财务指标（毛利率、ROE、EVA）4. 提出财务维度的建议和风险警示。",
    };
    cards.insert("CFO".to_string(), cfo_card.to_string());

    // COO 任务卡
    let coo_card = match scenario {
        BusinessScenario::StrategicPlanning =>
            "你是公司的COO。本次推演议题是战略规划/市场扩张。请：1. 评估现有产能与新市场目标的匹配度——需要新增多少产能？2. 建议产能建设的类型（手工线/半自动线/全自动线）和节奏 3. 分析供应链的承载能力——原材料供应是否有瓶颈？4. 评估不同扩张方案的运营风险 5. 测算运营效率指标和交付能力。",
        BusinessScenario::FinancialDecision =>
            "你是公司的COO。本次推演议题是融资/投资决策。请：1. 如果融资用于扩产，评估新增产能的投资效率和建设周期 2. 分析运营资金需求与融资方案的匹配度 3. 提出运营维度的资金使用优先级 4. 评估投资方案对运营效率的实际拉动效果。",
        BusinessScenario::OperationOptimization =>
            "你是公司的COO。本次推演议题是运营优化/成本控制。请：1. 用资本周转分析找出运营中的资金占用点——库存周转率、应收账期、生产周期 2. 分析每条生产线的效率和利用率 3. 提出具体的降本增效方案（供应链优化、库存管理改进、产线调整）4. 评估不同优化方案对交付能力的影响 5. 设定运营效率指标目标。",
        BusinessScenario::ProductLaunch =>
            "你是公司的COO。本次推演议题是新产品上市/产品规划。请：1. 分析新产品对现有产能的影响——需要新建产线还是改造现有产线？2. 计算产品转产的周期和成本 3. 评估原材料采购的风险——新材料的供应是否稳定？4. 提出产能建设的时间表和里程碑 5. 评估供应链的适配性。",
        BusinessScenario::CrisisManagement =>
            "你是公司的COO。本次推演议题是危机管理/企业经营困境。请：1. 立即评估运营中的止血点——哪些业务可以快速收缩？2. 提出库存变现的方案——成品库存快速出货、原材料订单可否取消或延期 3. 评估裁员/缩减产能的运营影响 4. 建议保留的核心业务和可剥离的非核心业务 5. 提出运营维度的紧急措施。",
        BusinessScenario::GeneralBusiness =>
            "你是公司的COO。请从运营角度分析当前经营议题：1. 评估产能与需求的匹配度 2. 分析研产销的平衡状况 3. 提出运营效率改进建议 4. 识别供应链和交付风险。",
    };
    cards.insert("COO".to_string(), coo_card.to_string());

    // CMO 任务卡
    let cmo_card = match scenario {
        BusinessScenario::StrategicPlanning =>
            "你是公司的CMO。本次推演议题是战略规划/市场扩张。请：1. 分析各市场的竞争格局和增长潜力 2. 用波士顿矩阵评估现有产品组合 3. 提出新市场的进入策略和营销投入规划 4. 分析竞争对手的可能反应和应对策略 5. 评估市场份额目标和可行性。",
        BusinessScenario::FinancialDecision =>
            "你是公司的CMO。本次推演议题是融资/投资决策。请：1. 从市场角度评估投资方案的市场前景 2. 分析营销投入与订单获取的关系——每M投入预计可激活多少订单 3. 评估市场开发投入的预期回报 4. 从客户/市场角度提供决策参考。",
        BusinessScenario::OperationOptimization =>
            "你是公司的CMO。本次推演议题是运营优化/成本控制。请：1. 分析各产品和各市场的利润率差异 2. 建议优化产品组合——收缩低利润产品、聚焦高利润产品 3. 用波士顿矩阵定位产品优先级 4. 评估降本方案对市场竞争力的影响——过度降本是否会影响品牌和客户？5. 提出市场维度的优化建议。",
        BusinessScenario::ProductLaunch =>
            "你是公司的CMO。本次推演议题是新产品上市/产品规划。请：1. 分析目标市场的需求规模和增长趋势 2. 确定新产品的市场定位和定价策略 3. 用波士顿矩阵分析新产品在现有产品组合中的角色 4. 制定上市推广计划——营销投入、渠道策略、时间表 5. 评估竞争对手的可能反应。",
        BusinessScenario::CrisisManagement =>
            "你是公司的CMO。本次推演议题是危机管理/企业经营困境。请：1. 评估当前市场的订单机会——哪些市场仍有需求？2. 提出快速变现的营销策略——折扣促销、清仓出货 3. 分析保留哪些市场、哪些市场可以暂时退出 4. 评估品牌声誉受损的风险和修复方案 5. 提出市场维度的紧急行动。",
        BusinessScenario::GeneralBusiness =>
            "你是公司的CMO。请从市场角度分析当前经营议题：1. 分析市场需求和竞争格局 2. 用波士顿矩阵评估产品组合 3. 提出营销策略和产品规划建议 4. 识别市场机会和风险。",
    };
    cards.insert("CMO".to_string(), cmo_card.to_string());

    // 管理咨询顾问 任务卡
    let consultant_card = match scenario {
        BusinessScenario::StrategicPlanning =>
            "你是独立的管理咨询顾问。本次推演议题是战略规划/市场扩张。请：1. 用杜邦公式分析当前ROE的驱动因素——是利润率高、周转快还是杠杆用得好？2. 对标行业最佳实践，评估企业当前的战略合理性 3. 用SWOT分析识别战略选择中的盲区 4. 评估不同战略方案的EVA——是否在为股东创造真实价值？5. 提出独立、客观的战略建议——不怕说真话。",
        BusinessScenario::FinancialDecision =>
            "你是独立的管理咨询顾问。本次推演议题是融资/投资决策。请：1. 用EVA评估融资/投资方案的经济合理性 2. 分析不同方案对公司长期竞争力的影响 3. 用财务杠杆分析评估风险承受能力 4. 对标行业通行的资本结构 5. 提出独立的专业建议。",
        BusinessScenario::OperationOptimization =>
            "你是独立的管理咨询顾问。本次推演议题是运营优化/成本控制。请：1. 用ABC成本法诊断各产品的真实成本结构 2. 对标行业运营效率基准 3. 识别运营中容易被忽略的隐性成本 4. 评估不同优化方案的长期和短期影响 5. 提出系统性改进建议——不只是头痛医头。",
        BusinessScenario::ProductLaunch =>
            "你是独立的管理咨询顾问。本次推演议题是新产品上市/产品规划。请：1. 用波士顿矩阵分析新产品在现有产品组合中的定位 2. 评估产品路线图与市场需求的匹配度 3. 对标同行新品上市的常见做法和成功概率 4. 分析产品组合的风险分散程度 5. 提出独立的产品战略建议。",
        BusinessScenario::CrisisManagement =>
            "你是独立的管理咨询顾问。本次推演议题是危机管理/企业经营困境。请：1. 客观诊断危机的根本原因——是战略失误、运营低效还是市场环境变化？2. 对标同行危机处理的经验和教训 3. 提出分阶段的拯救方案——紧急止血→稳定运营→战略转型 4. 评估不同拯救方案的成功概率和风险 5. 坦诚指出管理团队可能存在的决策盲区。",
        BusinessScenario::GeneralBusiness =>
            "你是独立的管理咨询顾问。请从专业第三方视角分析当前经营议题：1. 诊断核心问题 2. 用管理工具（杜邦分析、波士顿矩阵、ABC成本法）辅助分析 3. 对标最佳实践 4. 提出独立的专业建议。",
    };
    cards.insert("管理咨询顾问".to_string(), consultant_card.to_string());

    cards
}

#[derive(Debug, Deserialize)]
struct AnalyzeRequest { task: String, #[serde(default)] judgment: Option<String>, #[serde(default)] worry: Option<String>, #[serde(default)] unknown: Option<String>, #[serde(default)] reviewer: Option<String> }

#[derive(Debug, Clone, Serialize)]
struct SoulMatch { name: String, field: String, ismism_code: String, rationale: String }

fn pick_provider(state: &AppState) -> Result<Provider, (axum::http::StatusCode, Json<ApiError>)> {
    state.engine.gateway().pick_provider().ok_or_else(|| {
        tracing::error!("No LLM provider available");
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { error: "No LLM provider".into() }))
    })
}

/// Find the matching closing bracket starting from a given position, skipping bracket
/// characters inside JSON strings (honoring `"` and `\` escapes).
fn find_matching_bracket_from(text: &str, start_pos: usize, open: char, close: char) -> Option<(usize, usize)> {
    let start = start_pos;
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escape = false;

    for (i, ch) in text[start + 1..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    let abs_end = start + 1 + i;
                    return Some((start, abs_end));
                }
            }
        }
    }
    None
}

/// Find all matching bracket pairs in the text.
fn find_all_bracket_pairs(text: &str, open: char, close: char) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < bytes.len() {
        if let Some(start) = text[pos..].find(open) {
            let abs_start = pos + start;
            if let Some(pair) = find_matching_bracket_from(text, abs_start, open, close) {
                pairs.push(pair);
                pos = pair.1 + 1;
            } else {
                pos = abs_start + 1;
            }
        } else {
            break;
        }
    }
    pairs
}

/// Extract the last valid JSON array from text that may contain reasoning prose or multiple JSON fragments.
/// Returns the last array that parses successfully as JSON.
fn extract_json_array(text: &str) -> Option<&str> {
    let pairs = find_all_bracket_pairs(text, '[', ']');
    for (start, end) in pairs.iter().rev() {
        let candidate = &text[*start..=*end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Some(candidate);
        }
    }
    None
}

/// Extract the last valid JSON object from text that may contain reasoning prose or multiple JSON fragments.
/// Returns the last object that parses successfully as JSON.
fn extract_json(text: &str) -> Option<&str> {
    let pairs = find_all_bracket_pairs(text, '{', '}');
    for (start, end) in pairs.iter().rev() {
        let candidate = &text[*start..=*end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Some(candidate);
        }
    }
    None
}

/// 收集 LLM 流式响应的全部内容，同时记录第一个错误。
async fn collect_llm_stream(rx: &mut tokio::sync::mpsc::Receiver<Result<foundation::Chunk, foundation::FoundationError>>) -> (String, Option<String>) {
    // 预分配：LLM 响应常达数 KB，避免循环中多次 realloc。
    let mut raw = String::with_capacity(8 * 1024);
    let mut first_err: Option<String> = None;
    while let Some(r) = rx.recv().await {
        match r {
            Ok(c) => {
                if !c.content.is_empty() {
                    raw.push_str(&c.content);
                } else if let Some(ref rc) = c.reasoning_content {
                    raw.push_str(rc);
                }
            }
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e.to_string());
                }
                tracing::warn!("LLM stream chunk error: {}", e);
            }
        }
    }
    (raw, first_err)
}

fn build_soul_match(js: &serde_json::Value, all_souls: &[SoulListEntry]) -> Option<SoulMatch> {
    let name = js["name"].as_str()?;
    all_souls.iter().find(|e| e.name == name).map(|entry| SoulMatch {
        name: name.into(),

        field: entry.field.clone(),
        ismism_code: entry.ismism_code.clone(),
        rationale: js["rationale"].as_str().unwrap_or("").into(),
    })
}
async fn llm_call_json(state: &AppState, provider: Provider, system_prompt: Option<&str>, user_prompt: &str, temp: f64, max_tokens: u32, model: Option<&str>) -> Result<serde_json::Value, (axum::http::StatusCode, Json<ApiError>)> {
    let mut messages = Vec::new();
    if let Some(sys) = system_prompt {
        messages.push(PromptMessage { role: "system".into(), content: sys.into(), reasoning_content: None, tool_call_id: None, tool_calls: None });
    }
    messages.push(PromptMessage { role: "user".into(), content: user_prompt.into(), reasoning_content: None, tool_call_id: None, tool_calls: None });

    let req = LLMRequest { provider, prompt: Prompt { messages }, config: CallConfig { temperature: temp, max_tokens, stream: false, model: model.map(String::from), reasoning_effort: None, structured_output: None, thinking_enabled: None, tools: None, tool_choice: None } };
    let mut rx = state.engine.gateway().call(&req).map_err(|e| {
        tracing::error!("LLM call failed: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() }))
    })?;

    let mut resp = String::new();
    while let Some(result) = rx.recv().await {
        if let Ok(chunk) = result { resp.push_str(&chunk.content); }
    }
    // Extract JSON from response — DeepSeek reasoning models may prepend thinking text
    let json_str = extract_json(&resp).unwrap_or(&resp);
    tracing::debug!(len = resp.len(), json_len = json_str.len(), "llm_call_json response");
    Ok(serde_json::from_str(json_str).unwrap_or_else(|e| {
        // Safely truncate to avoid char boundary errors
        let trunc_len = resp.len().min(300);
        let safe_idx = (0..=trunc_len).rev().find(|&i| resp.is_char_boundary(i)).unwrap_or(0);
        tracing::warn!("llm_call_json parse failed: {} | raw[..{}]={}", e, safe_idx, &resp[..safe_idx]);
        serde_json::Value::default()
    }))
}

// ── Ismism Inference Engine ──
// 主义主义四位坐标体系：基于未明子的哲学分类学，从任务文本中推断
// 最可能的 ismism 坐标，用于提升匹配精度。

/// 任务中检测到的主义主义信号
#[derive(Debug, Default)]
struct TaskIsmismProfile {
    /// 每个 field (1-4) 的置信度，归一化到 [0, 1]
    field_weights: [f64; 4],
    /// 推断的 F-O-E-T 坐标（None 表示该维度不确定）
    inferred: Option<[u8; 4]>,
}

/// 主义主义术语 → 坐标映射。每个条目：(关键词, [F, O, E, T] 或 [F, 0, 0, 0])
/// 坐标来源于「主义主义完整目录_未明子原版」
const ISMISM_TERMS: &[(&str, [u8; 4])] = &[
    // ── Field 1: 形而下学（气学）──
    ("物理主义", [1, 1, 1, 0]),
    ("科学实在论", [1, 1, 0, 0]),
    ("建构论", [1, 1, 2, 0]),
    ("认知主义", [1, 1, 3, 0]),
    ("行为主义", [1, 1, 4, 0]),
    ("宗教实在论", [1, 2, 0, 0]),
    ("神创论", [1, 2, 1, 0]),
    ("偶像崇拜", [1, 2, 2, 0]),
    ("唯灵论", [1, 2, 3, 0]),
    ("反偶像崇拜", [1, 2, 4, 0]),
    ("唯我论", [1, 3, 0, 0]),
    ("伪唯心主义", [1, 3, 1, 0]),
    ("客观唯心", [1, 3, 1, 1]),
    ("主观唯心", [1, 3, 1, 2]),
    ("本真主义", [1, 3, 2, 0]),
    ("唯意志主义", [1, 3, 3, 0]),
    ("直觉主义", [1, 3, 4, 0]),
    ("平庸主义", [1, 4, 0, 0]),
    ("自然主义", [1, 4, 1, 0]),
    ("世俗人道", [1, 4, 2, 0]),
    ("心理主义", [1, 4, 3, 0]),
    ("庸俗主义", [1, 4, 4, 0]),

    // ── Field 2: 形而上学（道学）──
    ("在场形而上学", [2, 1, 0, 0]),
    ("普遍主义", [2, 1, 1, 0]),
    ("本质主义", [2, 1, 2, 0]),
    ("合理主义", [2, 1, 3, 0]),
    ("绝对主义", [2, 1, 4, 0]),
    ("辩证形而上学", [2, 2, 0, 0]),
    ("无限主义", [2, 2, 1, 0]),
    ("否定主义", [2, 2, 2, 0]),
    ("超验主义", [2, 2, 3, 0]),
    ("我思形而上学", [2, 3, 0, 0]),
    ("实体一元论", [2, 3, 1, 0]),
    ("理性主义", [2, 3, 2, 0]),
    ("反形而上学", [2, 4, 0, 0]),
    ("经验主义", [2, 4, 1, 0]),
    ("实证主义", [2, 4, 2, 0]),
    ("逻辑还原", [2, 4, 3, 0]),
    ("实用主义", [2, 4, 4, 0]),
    ("辩证唯物主义", [2, 4, 4, 2]),

    // ── Field 3: 观念论（心学）──
    ("现象学", [3, 1, 0, 0]),
    ("先验现象学", [3, 1, 1, 0]),
    ("象征主义", [3, 1, 2, 0]),
    ("生活世界", [3, 1, 3, 0]),
    ("德国观念论", [3, 2, 0, 0]),
    ("批判哲学", [3, 2, 1, 0]),
    ("知识学", [3, 2, 2, 0]),
    ("生存论", [3, 2, 3, 0]),
    ("辩证法", [3, 2, 4, 0]),
    ("存在主义", [3, 3, 1, 0]),
    ("尼采", [3, 3, 3, 0]),
    ("符号学", [3, 4, 0, 0]),
    ("结构主义", [3, 4, 1, 0]),
    ("后结构主义", [3, 4, 2, 0]),
    ("差异的辩证法", [3, 4, 3, 0]),
    ("解释学", [3, 4, 4, 0]),
    ("精神分析", [3, 4, 4, 4]),

    // ── Field 4: 实践 · 辩证唯物主义 ──
    ("政治经济学批判", [4, 1, 0, 0]),
    ("资本主义", [4, 1, 1, 0]),
    ("文化霸权", [4, 1, 3, 4]),
    ("意识形态批判", [4, 1, 0, 0]),
    ("列宁", [4, 1, 4, 3]),
    ("组织建设", [4, 2, 0, 0]),
    ("国际主义", [4, 2, 3, 0]),
    ("理想社会", [4, 3, 0, 0]),
    ("生产活动", [4, 3, 2, 0]),
    ("乌托邦", [4, 4, 0, 0]),
    ("去人类中心", [4, 4, 2, 0]),

    // ── 跨域术语 ──
    ("康德", [3, 2, 1, 0]),
    ("黑格尔", [3, 2, 4, 0]),
    ("海德格尔", [3, 2, 3, 0]),
    ("胡塞尔", [3, 1, 1, 0]),
    ("马克思", [2, 4, 4, 2]),
    ("毛泽东", [4, 1, 3, 0]),
    ("萨特", [3, 3, 1, 2]),
    ("维特根斯坦", [3, 4, 4, 3]),
    ("拉康", [3, 4, 4, 4]),
    ("德勒兹", [3, 4, 3, 0]),
    ("福柯", [3, 4, 2, 0]),
    ("葛兰西", [4, 1, 3, 4]),
    ("柏拉图", [2, 1, 2, 0]),
    ("亚里士多德", [2, 1, 2, 3]),
    ("苏格拉底", [2, 1, 2, 1]),
    ("笛卡尔", [2, 3, 2, 0]),
    ("斯宾诺莎", [2, 3, 1, 0]),
    ("莱布尼茨", [2, 3, 1, 0]),
    ("叔本华", [1, 3, 3, 4]),
    ("柏格森", [1, 3, 4, 0]),
    ("巴门尼德", [2, 1, 2, 0]),
    ("赫拉克利特", [2, 1, 1, 4]),
    ("毕达哥拉斯", [2, 1, 3, 0]),
    ("阿多诺", [3, 2, 4, 2]),
    ("齐泽克", [3, 4, 4, 4]),
];

/// 从任务文本推断 ismism 坐标
fn infer_task_ismism(task: &str) -> TaskIsmismProfile {
    let task_lower = task.to_lowercase();
    let mut field_hits = [0u32; 4];
    let mut weighted_coords = [0f64; 4];
    let mut total_weight = 0f64;

    for &(term, coords) in ISMISM_TERMS {
        if task_lower.contains(&term.to_lowercase()) {
            let idx = coords[0] as usize - 1; // field 1→index 0, etc.
            field_hits[idx] += 1;
            // Weight: each matched dimension contributes, dim-1 (field) = ×2 weight
            let term_weight = if coords[3] != 0 { 2.0 } else if coords[2] != 0 { 1.5 } else { 1.0 };
            for d in 0..4 {
                if coords[d] != 0 {
                    weighted_coords[d] += coords[d] as f64 * term_weight;
                }
            }
            total_weight += term_weight;
        }
    }

    let total_hits: u32 = field_hits.iter().sum();
    let field_weights: [f64; 4] = if total_hits > 0 {
        [
            field_hits[0] as f64 / total_hits as f64,
            field_hits[1] as f64 / total_hits as f64,
            field_hits[2] as f64 / total_hits as f64,
            field_hits[3] as f64 / total_hits as f64,
        ]
    } else {
        [0.25, 0.25, 0.25, 0.25]
    };

    let inferred = if total_weight > 0.0 {
        Some([
            (weighted_coords[0] / total_weight).round().clamp(1.0, 4.0) as u8,
            (weighted_coords[1] / total_weight).round().clamp(0.0, 4.0) as u8,
            (weighted_coords[2] / total_weight).round().clamp(0.0, 4.0) as u8,
            (weighted_coords[3] / total_weight).round().clamp(0.0, 4.0) as u8,
        ])
    } else {
        None
    };

    tracing::debug!(
        "Ismism inference: task=\"{:.80}\" fields={:?} inferred={:?}",
        task, field_weights, inferred
    );

    TaskIsmismProfile { field_weights, inferred }
}

/// 解析 ismism code（如 "1-2-3-4"）为 [u8; 4]
fn parse_ismism(code: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.len() < 4 { return None; }
    let nums: Vec<u8> = parts.iter().filter_map(|p| p.parse().ok()).collect();
    (nums.len() >= 4).then(|| [nums[0], nums[1], nums[2], nums[3]])
}

/// 计算魂的 ismism 坐标与任务推断坐标的邻近度 (0.0 ~ 1.0)
fn ismism_proximity_score(profile: &TaskIsmismProfile, soul: &SoulListEntry) -> f64 {
    let soul_code = match parse_ismism(&soul.ismism_code) {
        Some(c) => c,
        None => return 0.0,
    };

    // 在场域权重：任务所在的 field 越高，同 field 的魂得分越高
    let soul_field_idx = soul_code[0].saturating_sub(1) as usize;
    if soul_field_idx >= 4 { return 0.0; }
    let field_weight = profile.field_weights[soul_field_idx];

    // 如果有精确的坐标推断，计算各维度邻近度
    let coord_prox = if let Some(inferred) = profile.inferred {
        let mut sim = 0.0;
        // Field dim (×2 weight — most important)
        let f_diff = (soul_code[0] as f64 - inferred[0] as f64).abs();
        sim += (1.0 - f_diff / 4.0) * 0.40;
        // 其余三维
        for d in 1..4 {
            if inferred[d] != 0 && soul_code[d] != 0 {
                let diff = (soul_code[d] as f64 - inferred[d] as f64).abs();
                sim += (1.0 - diff / 4.0) * 0.20;
            }
        }
        sim
    } else {
        // 无精确推断，纯 field 权重
        field_weight
    };

    // Mix: 60% field weight + 40% coordinate proximity
    field_weight * 0.6 + coord_prox * 0.4
}

/// 工具意识分数：魂越明确承认自己的边界和结构性位置，分数越高。
/// 承认工具性不是示弱——说出"我被这样构成"的同时已经开始拆解。
fn tool_awareness_score(soul: &SoulListEntry) -> f64 {
    let decl = soul.self_declare.to_lowercase();
    if decl.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;

    // 有明确的"我不做"——边界意识
    let boundary_markers = ["我不做", "我不", "我不能", "我不擅长", "不是我的"];
    for m in &boundary_markers {
        if decl.contains(m) {
            score += 0.25;
            break;
        }
    }

    // 有明确的"我做"/"我是"——位置声明
    let position_markers = ["我做", "我是", "我的位置", "我的方法", "我的核心功能", "我做的事"];
    for m in &position_markers {
        if decl.contains(m) {
            score += 0.25;
            break;
        }
    }

    // 有"互补"——知道谁补自己的盲区
    if decl.contains("互补") {
        score += 0.20;
    }

    // 有具体人名/学派名——不是抽象描述，是锚在具体思想上
    let concrete_names = ["马克思", "列宁", "毛泽东", "费曼", "庄子", "尼采", "葛兰西",
        "黑格尔", "胡塞尔", "鲁迅", "孔子", "稻盛和夫", "波伏娃", "未明子", "韩炳哲"];
    let name_count = concrete_names.iter().filter(|&&n| decl.contains(&n.to_lowercase())).count();
    if name_count >= 3 {
        score += 0.20;
    } else if name_count >= 1 {
        score += 0.10;
    }

    // self_declare 长度——越长越具体（但设上限避免过长也不加分）
    let len_score = (decl.len() as f64 / 200.0).min(0.10);
    score += len_score;

    score.min(1.0)
}

/// 统计任务文本命中魂的领域术语的次数
fn count_domain_term_hits(task_lower: &str, soul: &SoulListEntry) -> usize {
    let mut hits = 0;
    for domain in &soul.domains {
        for word in domain.split(&['/', '、', ',', '，'][..]) {
            let w = word.trim();
            if w.len() >= 2 && task_lower.contains(&w.to_lowercase()) {
                hits += 1;
                break;
            }
        }
    }
    // Also check field name
    for word in soul.field.split(&['/', '、', '.', '·'][..]) {
        let w = word.trim();
        if w.len() >= 2 && task_lower.contains(&w.to_lowercase()) {
            hits += 1;
            break;
        }
    }
    hits
}

async fn analyze_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AnalyzeRequest>,
) -> Sse<UnboundedReceiverStream<Result<Event, std::convert::Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event, std::convert::Infallible>>();

    tokio::spawn(async move {
        let send = |data: String| {
            let _ = tx.send(Ok(Event::default().data(data)));
        };

        let trunc_len = body.task.len().min(50);
        let safe_idx = (0..=trunc_len).rev().find(|&i| body.task.is_char_boundary(i)).unwrap_or(0);
        tracing::info!("Starting task analysis for: {}", &body.task[..safe_idx]);

        // Entry classification
        let has_specific = body.task.contains("我") && (body.task.contains("做了") || body.task.contains("正在") || body.task.contains("经历过") || body.task.contains("我公司") || body.task.contains("我的项目") || body.task.contains("我的工厂"));
        let has_first_person = body.task.starts_with("我");
        let has_concrete = !body.task.contains("通常") && !body.task.contains("一般") && (body.task.contains("上次") || body.task.contains("最近") || body.task.contains("今天") || body.task.contains("昨天") || body.task.contains("这周"));
        let score = [has_specific, has_first_person, has_concrete].iter().filter(|&&x| x).count();

        if score >= 2 {
            send(serde_json::json!({
                "phase": "practice_opening"
            }).to_string());
            send(serde_json::json!({
                "phase": "done",
                "response": {
                    "entry_type": "practice_opening",
                    "matched_souls": serde_json::json!([]),
                    "recommended_mode": "practice_opening",
                    "review": { "verdict": "practice_opening", "checks": [], "notes": "", "reviewer": "" },
                    "task_cards": {}
                }
            }).to_string());
            return;
        }

        let entry_type = if score == 1 { "hybrid" } else { "conventional" };

        let provider = match pick_provider(&state) {
            Ok(p) => p,
            Err(_) => {
                send(serde_json::json!({ "phase": "error", "message": "无可用的 AI provider" }).to_string());
                send(serde_json::json!({
                    "phase": "done",
                    "response": {
                        "entry_type": "conventional",
                        "matched_souls": [],
                        "recommended_mode": "single",
                        "review": { "verdict": "pass", "checks": [], "notes": "No LLM provider available", "reviewer": "" },
                        "task_cards": {}
                    }
                }).to_string());
                return;
            }
        };

        let all_souls = match state.registry.list_souls(&foundation::IsmismFilter::default()) {
            Ok(s) => s,
            Err(_) => {
                send(serde_json::json!({ "phase": "error", "message": "魂列表加载失败" }).to_string());
                send(serde_json::json!({
                    "phase": "done",
                    "response": {
                        "entry_type": "conventional",
                        "matched_souls": [],
                        "recommended_mode": "single",
                        "review": { "verdict": "pass", "checks": [], "notes": "Failed to list souls", "reviewer": "" },
                        "task_cards": {}
                    }
                }).to_string());
                return;
            }
        };
        let task_lower = body.task.to_lowercase();

        // ── Phase: classifying ──
        send(serde_json::json!({ "phase": "classifying", "entry_type": entry_type }).to_string());
        // 立即进入匹配阶段，避免前端卡在入口分流
        send(serde_json::json!({ "phase": "matching" }).to_string());

        // ── Step 1: Algorithmic matching ──
        // 1a. FT semantic search
        let ft_results = state.registry.search_souls(&body.task).unwrap_or_default();
        let ft_scores: std::collections::HashMap<String, f64> = ft_results.iter()
            .map(|m| (m.entry.name.clone(), m.relevance))
            .collect();

        // 1b. Task → Ismism inference: scan for known philosophical terms
        let task_ismism = infer_task_ismism(&body.task);

        // 1c. Multi-factor composite scoring
        let mut scored: Vec<(&SoulListEntry, f64, &str)> = all_souls.iter().map(|s| {
            let kw_hits = s.trigger_keywords.iter()
                .filter(|kw| task_lower.contains(&kw.to_lowercase()))
                .count();
            let ft_score = ft_scores.get(&s.name).copied().unwrap_or(0.0);
            let ismism_prox = ismism_proximity_score(&task_ismism, s);
            let domain_hits = count_domain_term_hits(&task_lower, s);
            let tool_score = tool_awareness_score(s);

            // Composite: Ismism(40%) + Tool(15%) + Domain(20%) + Keywords(10%) + FT(15%)
            let composite = ismism_prox * 0.40
                + tool_score * 0.15
                + (domain_hits as f64).min(3.0) / 3.0 * 0.20
                + (kw_hits as f64).min(3.0) / 3.0 * 0.10
                + ft_score * 0.15;

            (s, composite, if ismism_prox > 0.4 { "ismism" } else if kw_hits > 0 { "keyword" } else if ft_score > 0.2 { "semantic" } else { "fallback" })
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.0.summon_count.cmp(&a.0.summon_count)));

        // Dynamic top-N selection: stop at relevance floor + elbow + hard cap
        let relevance_floor = 0.05; // ignore souls with near-zero composite
        let hard_cap = 5usize;      // never match more than 5 souls, target ~4
        let mut take_n = 0usize;
        let mut prev_score = f64::MAX;
        for (i, (_, score, _)) in scored.iter().enumerate() {
            if *score < relevance_floor { break; }
            // Elbow detection: if score drops >50% from previous, stop here
            if i > 0 && i >= 2 && prev_score > 0.0 && (*score / prev_score) < 0.5 {
                break;
            }
            if i >= hard_cap { break; }
            take_n = i + 1;
            prev_score = *score;
        }
        // Floor: at least 1 soul if any scored above relevance floor
        take_n = take_n.max(if scored.first().map(|s| s.1).unwrap_or(0.0) >= relevance_floor { 1 } else { 1 })
                       .min(scored.len());

        let souls: Vec<SoulMatch> = scored.iter().take(take_n).map(|(s, composite, match_type)| {
            let kw_hits = s.trigger_keywords.iter()
                .filter(|kw| task_lower.contains(&kw.to_lowercase()))
                .count();
            let ismism_prox = ismism_proximity_score(&task_ismism, s);
                        let rationale = match *match_type {
                "keyword" => format!("命中 {} 个关键词 | 坐标邻近度 {:.0}% | 综合分 {:.3}", kw_hits, ismism_prox*100.0, composite),
                "ismism" => format!("坐标邻近度 {:.0}% | 综合分 {:.3}", ismism_prox*100.0, composite),
                "semantic" => format!("全文相关性 | 坐标邻近度 {:.0}% | 综合分 {:.3}", ismism_prox*100.0, composite),
                                _ => format!("综合相关性 | 坐标邻近度 {:.0}% | 综合分 {:.3}", ismism_prox*100.0, composite),
            };
            SoulMatch {
                name: s.name.clone(),
                field: s.field.clone(),
                ismism_code: s.ismism_code.clone(),
                rationale,
            }
        }).collect();

        // ── Ismism-based mode determination ──

        // Calculate Ismism diversity score (0.0 = identical, 1.0 = maximum diversity)
        fn calc_ismism_diversity(codes: &[[u8; 4]]) -> (f64, usize, usize) {
            if codes.len() < 2 {
                return (0.0, 0, 0);
            }

            let mut total_field_diff = 0.0;
            let mut total_ontology_diff = 0.0;
            let mut total_epistemology_diff = 0.0;
            let mut total_teleology_diff = 0.0;
            let comparisons = codes.len() * (codes.len() - 1) / 2;
            let mut high_field_count = 0;
            let mut high_ontology_count = 0;
            let mut high_epistemology_count = 0;
            let mut high_teleology_count = 0;

            for i in 0..codes.len() {
                for j in (i+1)..codes.len() {
                    let diff0 = codes[i][0].abs_diff(codes[j][0]) as f64 / 3.0;
                    let diff1 = codes[i][1].abs_diff(codes[j][1]) as f64 / 3.0;
                    let diff2 = codes[i][2].abs_diff(codes[j][2]) as f64 / 3.0;
                    let diff3 = codes[i][3].abs_diff(codes[j][3]) as f64 / 3.0;
                    total_field_diff += diff0;
                    total_ontology_diff += diff1;
                    total_epistemology_diff += diff2;
                    total_teleology_diff += diff3;
                    if diff0 >= 0.4 { high_field_count += 1; }
                    if diff1 >= 0.4 { high_ontology_count += 1; }
                    if diff2 >= 0.4 { high_epistemology_count += 1; }
                    if diff3 >= 0.4 { high_teleology_count += 1; }
                }
            }

            let avg_field = total_field_diff / comparisons as f64;
            let _avg_ontology = total_ontology_diff / comparisons as f64;
            let _avg_epistemology = total_epistemology_diff / comparisons as f64;
            let _avg_teleology = total_teleology_diff / comparisons as f64;

            // Weighted diversity: teleology > epistemology > ontology > field
            let diversity = avg_field * 0.15 + _avg_ontology * 0.20 + _avg_epistemology * 0.25 + _avg_teleology * 0.40;

            // Count how many dimensions have high diversity
            let high_dimensions = [high_field_count, high_ontology_count, high_epistemology_count, high_teleology_count]
                .iter().filter(|&&c| c >= comparisons / 2).count();

            (diversity, high_dimensions, comparisons)
        }

        let mode = if souls.len() <= 1 {
            "single".to_string()
        } else {
            // Parse ismism codes from matched souls
            let ismism_codes: Vec<[u8; 4]> = souls.iter()
                .filter_map(|s| parse_ismism(&s.ismism_code))
                .collect();

            let (diversity, high_dims, comparisons) = calc_ismism_diversity(&ismism_codes);

            tracing::debug!(
                "Ismism mode analysis: souls={} codes={} diversity={:.3} high_dims={}/4 comparisons={}",
                souls.len(), ismism_codes.len(), diversity, high_dims, comparisons
            );

            // Mode determination based on Ismism diversity:
            // - Single: 1 soul, or only 1 parseable code, or diversity < 0.2
            // - Conference: default for 2-5 souls with moderate diversity
            // - Debate: >= 3 souls AND all 4 dimensions diverge AND diversity >= 0.6
            //   (high bar: genuine philosophical opposition, not just different perspectives)
            //
            // Rationale: 2 souls from different fields can still collaborate in conference;
            // debate requires at least 3 genuinely opposed positions.
            if diversity < 0.2 || ismism_codes.len() < 2 {
                "single".to_string()
            } else if ismism_codes.len() >= 3 && diversity >= 0.6 && high_dims == 4 {
                "debate".to_string()
            } else {
                "conference".to_string()
            }
        };

        // ── Phase: matched ──
        send(serde_json::json!({
            "phase": "matched",
            "souls": souls.iter().map(|s| serde_json::json!({
                "name": s.name, "field": s.field, "ismism_code": s.ismism_code, "rationale": s.rationale,
            })).collect::<Vec<_>>(),
            "mode": mode,
        }).to_string());

        // ── Step 2: Banner lord review ──
        let reviewer_name = body.reviewer.clone()
            .unwrap_or_else(|| std::env::var("AIONUI_REVIEWER_SOUL").unwrap_or_else(|_| "未明子".to_string()));
        let mut task_cards: std::collections::HashMap<String, String> = Default::default();

        // ── Phase: reviewing (stream before waiting for LLM) ──
        send(serde_json::json!({
            "phase": "reviewing",
            "reviewer": reviewer_name,
        }).to_string());

        let (verdict, checks, notes, verified_souls, missing_perspectives) = if let Ok(banner_lord) = state.registry.get_soul(&reviewer_name) {
            let candidate_profiles: Vec<SoulProfile> = souls.iter()
                .filter_map(|s| state.registry.get_soul(&s.name).ok())
                .collect();

            let provider = state.engine.gateway().pick_provider()
                .ok_or_else(|| "No LLM provider available".to_string());

            let domain = &state.config.domain;
            let coord_label = domain.terms.get("coord_label").map(|s| s.as_str()).unwrap_or("坐标");
            let agent_noun = domain.terms.get("agent_noun").map(|s| s.as_str()).unwrap_or("魂");
            let banner_lord_title = domain.terms.get("banner_lord").map(|s| s.as_str()).unwrap_or("幡主");

            let result = match provider {
                Ok(provider) => {
                    let review_system = format!(
                        "{}{}你是{}，{}{}。你作为{banner_lord_title}审查官，需要完成两项任务：\n\
                         1. 审查候选{agent_noun}是否适合这个任务——不适合的要去掉或替换\n\
                         2. 为每个确定使用的{agent_noun}分派一个**差异化的子问题**——不是所有人分析同一个问题，\
                         而是把你的总任务拆解成每个{agent_noun}最擅长回答的那一个侧面\n\n\
                         不读取文件——所有上下文已在 prompt 中。",
                        banner_lord.summon_prompt, banner_lord.name, coord_label, banner_lord.ismism_code,
                        banner_lord_title = banner_lord_title,
                        agent_noun = agent_noun,
                    );

                    let mut candidates_info = String::new();
                    for p in &candidate_profiles {
                        let exclude_str = p.exclude_scenarios.join("、");
                        candidates_info.push_str(&format!(
                            "- **{}** [{}] {}=\"{}\" self_declare=\"{}\" exclude_scenarios=\"{}\"\n",
                            p.name, p.field, coord_label, p.ismism_code,
                            if p.self_declare.is_empty() { "无" } else { &p.self_declare },
                            if p.exclude_scenarios.is_empty() { "无" } else { &exclude_str }
                        ));
                    }

                    // Domain-specific role descriptions for differential task assignment
                    let dims = &domain.coordinate.dimensions;
                    let d0 = dims.first().map(|d| d.name.as_str()).unwrap_or("维度1");
                    let d1 = dims.get(1).map(|d| d.name.as_str()).unwrap_or("维度2");
                    let d2 = dims.get(2).map(|d| d.name.as_str()).unwrap_or("维度3");
                    let d3 = dims.get(3).map(|d| d.name.as_str()).unwrap_or("维度4");
                    let role_guide = format!(
                        "- 利用坐标差异——{d0}在前的{agent_noun}做地基（\"这是什么\"），\
                         {d1}在前的{agent_noun}做边界（\"这看不到什么\"），\
                         {d2}在前的{agent_noun}做自反（\"这个问法本身有什么问题\"），\
                         {d3}在前的{agent_noun}做实践（\"怎么落地\"）",
                        d0 = d0, d1 = d1, d2 = d2, d3 = d3, agent_noun = agent_noun
                    );

                    let review_user = format!(
                        "## 总任务\n{}\n\n## 使用者预设\n判断：{}\n担忧：{}\n未知：{}\n\n## 候选{agent_noun}\n{}\n\n\
                         ## 全{agent_noun}库（供补位参考）\n{}\n\n\
                         ## 你的两阶段任务\n\n\
                         ### 第一阶段：审查{agent_noun}组合\n\
                         逐{agent_noun}检查：领域覆盖、{coord_label}定位、{agent_noun}间互补、视角缺失。\
                         裁决：pass / conditional / reject\n\n\
                         **CRITICAL：如果裁决为 conditional 且需要补位，你必须把补位{agent_noun}的名字也加入 verified_souls 数组中。\
                         verified_souls 是上场名单的唯一数据源，只写在 missing_perspectives 里的{agent_noun}不会上场！**\n\n\
                         ### 第二阶段：差异化任务分派\n\
                         为每个确认使用的{agent_noun}分配一个**只有他能回答好的子问题**。原则：\n\
                         {role_guide}\n\
                         - 每个子问题要具体（\"请回答：X在Y条件下的Z\"），不要\"请分析\"这种空指令\n\n\
                         返回JSON：\
                         {{\"verdict\":\"pass|conditional|reject\",\
                         \"verified_souls\":[\"{agent_noun}名\"],\
                         \"task_cards\":{{\"{agent_noun}名\":\"专属子问题\"}},\
                         \"checks\":[\"审查结果\"],\
                         \"notes\":\"审查备注\",\
                         \"missing_perspectives\":[\"缺失视角\"],\
                         \"boundary_risks\":[\"边界风险\"]}}",
                        &body.task,
                        body.judgment.as_deref().unwrap_or(""),
                        body.worry.as_deref().unwrap_or(""),
                        body.unknown.as_deref().unwrap_or(""),
                        candidates_info,
                        &all_souls.iter().map(|s| format!("{} [{}] {}={}", s.name, s.field, coord_label, s.ismism_code)).collect::<Vec<_>>().join("\n"),
                        agent_noun = agent_noun,
                        coord_label = coord_label,
                        role_guide = role_guide,
                    );

                    let messages = vec![
                        PromptMessage {
                            role: "system".into(),
                            content: review_system,
                            reasoning_content: None,
                            tool_call_id: None,
                            tool_calls: None,
                        },
                        PromptMessage {
                            role: "user".into(),
                            content: review_user,
                            reasoning_content: None,
                            tool_call_id: None,
                            tool_calls: None,
                        },
                    ];

                    let req = LLMRequest {
                        provider,
                        prompt: Prompt { messages },
                        config: CallConfig {
                            temperature: 0.3,
                            max_tokens: 4096,
                            stream: false,
                            model: None,
                            reasoning_effort: None,
                            structured_output: None,
                            thinking_enabled: None,
                            tools: None,
                            tool_choice: None,
                        },
                    };

                    let mut rx = match state.engine.gateway().call(&req) {
                        Ok(rx) => rx,
                        Err(e) => {
                            let error_msg = format!("LLM call failed: {}", e);
                            tracing::warn!("{}", error_msg);
                            send(serde_json::json!({
                                "phase": "analysis_content",
                                "stage": "review",
                                "source": reviewer_name,
                                "content": error_msg,
                                "is_partial": false
                            }).to_string());
                            send(serde_json::json!({
                                "phase": "done",
                                "response": {
                                    "entry_type": "conventional",
                                    "matched_souls": all_souls.iter().take(3).map(|s| s.name.clone()).collect::<Vec<_>>(),
                                    "recommended_mode": "single",
                                    "review": { "verdict": "pass", "checks": [], "notes": "Review failed", "reviewer": "" },
                                    "task_cards": {}
                                }
                            }).to_string());
                            return;
                        }
                    };
                    let mut resp = String::new();
                    let mut chunk_buffer = String::new();
                    let mut buffer_size = 0usize;
                    while let Some(result) = rx.recv().await {
                        match result {
                            Ok(chunk) => {
                                let content = if !chunk.content.is_empty() {
                                    &chunk.content
                                } else if let Some(ref rc) = chunk.reasoning_content {
                                    rc
                                } else {
                                    continue;
                                };
                                if !content.is_empty() {
                                    resp.push_str(content);
                                    chunk_buffer.push_str(content);
                                    buffer_size += content.len();
                                    if buffer_size >= 20 {
                                        // take() 直接拿走 String 堆数据 (O(1))，
                                        // 替代 clone()+clear() 的整块拷贝 (O(n))。
                                        let content = std::mem::take(&mut chunk_buffer);
                                        let payload = serde_json::json!({
                                            "phase": "analysis_content",
                                            "stage": "review",
                                            "source": reviewer_name,
                                            "content": content,
                                            "is_partial": true
                                        }).to_string();
                                        send(payload);
                                        buffer_size = 0;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("LLM stream chunk error: {}", e);
                            }
                        }
                    }
                    if !chunk_buffer.is_empty() {
                        let payload = serde_json::json!({
                            "phase": "analysis_content",
                            "stage": "review",
                            "source": reviewer_name,
                            "content": chunk_buffer,
                            "is_partial": true
                        }).to_string();
                        send(payload);
                    }
                    let payload = serde_json::json!({
                        "phase": "analysis_content",
                        "stage": "review",
                        "source": reviewer_name,
                        "content": "",
                        "is_partial": false,
                        "is_done": true
                    }).to_string();
                    send(payload);
                    let json_str = extract_json(&resp).unwrap_or(&resp);
                    serde_json::from_str(json_str).unwrap_or_default()
                }
                Err(e) => {
                    tracing::error!("Banner lord review failed to pick provider: {}", e);
                    serde_json::Value::default()
                }
            };

            if let Some(cards) = result["task_cards"].as_object() {
                for (k, v) in cards {
                    if let Some(val) = v.as_str() {
                        task_cards.insert(k.clone(), val.to_string());
                    }
                }
            }
            let missing_perspectives: Vec<String> = result["missing_perspectives"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let boundary_risks: Vec<String> = result["boundary_risks"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let verified_souls: Vec<String> = result["verified_souls"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let v = result["verdict"].as_str().unwrap_or("pass").to_string();
            let c: Vec<String> = result["checks"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let n = if !missing_perspectives.is_empty() {
                format!("{} | 缺失视角：{} | 边界风险：{}",
                    result["notes"].as_str().unwrap_or(""),
                    missing_perspectives.join("、"),
                    boundary_risks.join("、"))
            } else {
                result["notes"].as_str().unwrap_or("").to_string()
            };
            (v, c, n, verified_souls, missing_perspectives)
        } else {
            ("pass".to_string(), vec!["审查官不可用，默认通过".into()], String::new(), vec![], vec![])
        };

        // ── Phase: review_done ──
        send(serde_json::json!({
            "phase": "review_done",
            "review": {
                "verdict": verdict,
                "checks": checks,
                "notes": notes,
                "reviewer": reviewer_name,
            },
            "task_cards": task_cards,
        }).to_string());

        // ── Step 3: 一审结束，以审查官推荐阵容为准 ──
        // 审查官一审即终裁。verified_souls 为最终阵容，task_cards 为专属子任务。
        let final_verdict = verdict.clone();
        let final_checks = checks.clone();
        let final_notes = notes.clone();
        let final_missing_perspectives = missing_perspectives.clone();
        let mut final_verified_souls = verified_souls.clone();

        // ── 补位兜底：审查官标注了缺失视角但未加入 verified_souls 时，自动从魂库匹配 ──
        if !final_missing_perspectives.is_empty() {
            let existing: std::collections::HashSet<String> = final_verified_souls.iter().cloned().collect();
            let mut fills: Vec<String> = Vec::new();
            for perspective in &final_missing_perspectives {
                let perspective_lower = perspective.to_lowercase();
                for entry in &all_souls {
                    if existing.contains(&entry.name) || fills.contains(&entry.name) {
                        continue;
                    }
                    let name_lower = entry.name.to_lowercase();
                    let field_lower = entry.field.to_lowercase();
                    let tags_lower: String = entry.tags.join(" ").to_lowercase();
                    let domains_lower: String = entry.domains.join(" ").to_lowercase();
                    if perspective_lower.contains(&name_lower)
                        || perspective_lower.contains(&field_lower)
                        || tags_lower.split_whitespace().any(|t| perspective_lower.contains(t))
                        || domains_lower.split_whitespace().any(|d| perspective_lower.contains(d))
                    {
                        tracing::info!(
                            "Auto-fill missing perspective: '{}' → soul '{}' [{}]",
                            perspective, entry.name, entry.field
                        );
                        fills.push(entry.name.clone());
                        break;
                    }
                }
            }
            final_verified_souls.extend(fills);
        }

        // Cap total souls to prevent matching explosion
        let max_souls = 5usize;
        final_verified_souls.truncate(max_souls);

        // ── Step 4: 以审查官 verified_souls 为唯一权威 ──
        // 审查官是唯一权威。算法匹配只是参考，终裁的 verified_souls 决定最终阵容。
        let final_souls = if !final_verified_souls.is_empty() {
            let chosen: Vec<SoulMatch> = final_verified_souls.iter()
                .filter_map(|name| {
                    // 先在算法匹配结果中找
                    if let Some(s) = souls.iter().find(|s| s.name == *name).cloned() {
                        return Some(s);
                    }
                    // 算法没匹配到但审查官指定了——从 registry 补齐
                    all_souls.iter().find(|e| e.name == *name).map(|entry| {
                        let ismism_prox = ismism_proximity_score(&task_ismism, entry);
                        SoulMatch {
                            name: entry.name.clone(),
                            field: entry.field.clone(),
                            ismism_code: entry.ismism_code.clone(),
                            rationale: format!("审查官指定 | 领域 {} | 坐标邻近度 {:.0}%", entry.field, ismism_prox * 100.0),
                        }
                    })
                })
                .collect();

            if chosen.is_empty() {
                tracing::warn!("final_verified_souls={:?} not found in registry or matched, keeping all", final_verified_souls);
                souls
            } else {
                tracing::info!(
                    "Banner lord final authority: final_verified_souls={:?}, final {} souls (algorithm matched {})",
                    final_verified_souls, chosen.len(), souls.len()
                );
                chosen
            }
        } else if final_verdict == "pass" {
            tracing::info!("No verified_souls from review, pass → using all {} algorithm-matched souls", souls.len());
            souls
        } else {
            // ── Phase: adjusting ──
            send(serde_json::json!({ "phase": "adjusting" }).to_string());

            let adjustment_prompt = format!(
                "## 任务\n{}\n\n## 当前魂组合\n{}\n\n## 幡主审查结果\n裁决：{}\n{}\n备注：{}\n\n## 指令\n根据审查结果调整魂组合。如果是条件通过——增删魂以满足约束。如果是拒绝——完全重新匹配。\n返回JSON：{{\"souls\":[{{\"name\":\"魂名\",\"rationale\":\"调整理由\"}}]}}",
                body.task,
                souls.iter().map(|s| format!("- {} [{}] {}", s.name, s.field, s.rationale)).collect::<Vec<_>>().join("\n"),
                final_verdict, final_checks.join("\n"), final_notes
            );
            let provider3 = provider.clone();
            match llm_call_json(&state, provider3, None, &adjustment_prompt, 0.3, 4096, None).await {
                Ok(adjusted) => {
                    let adjusted_souls = adjusted["souls"].as_array()
                        .map(|arr| arr.iter().filter_map(|s| build_soul_match(s, &all_souls)).collect())
                        .unwrap_or(souls.clone());
                    if adjusted_souls.iter().any(|s| !souls.iter().any(|orig| orig.name == s.name))
                        || souls.iter().any(|orig| !adjusted_souls.iter().any(|s| s.name == orig.name))
                    {
                        if !adjusted_souls.is_empty() {
                            let adjusted_profiles: Vec<SoulProfile> = adjusted_souls.iter()
                                .filter_map(|s| state.registry.get_soul(&s.name).ok())
                                .collect();
                            if let Ok(banner_lord) = state.registry.get_soul(&reviewer_name) {
                                let rs = format!(
                                    "{}你是{}，ismism坐标{}。你作为幡主审查官，为调整后的魂组合分配差异化子问题。",
                                    banner_lord.summon_prompt, banner_lord.name, banner_lord.ismism_code
                                );
                                let p_info: Vec<String> = adjusted_profiles.iter().map(|p| {
                                    format!("- {} [{}] ismism={}", p.name, p.field, p.ismism_code)
                                }).collect();
                                let ru = format!(
                                    "## 总任务\n{}\n\n## 调整后的魂组合\n{}\n\n为每个魂分配一个只有他能回答好的子问题。\n返回JSON：{{\"task_cards\":{{\"魂名\":\"专属子问题\"}}}}",
                                    body.task, p_info.join("\n")
                                );
                                if let Ok(tc) = llm_call_json(&state, provider.clone(), Some(&rs), &ru, 0.3, 4096, None).await {
                                    if let Some(cards) = tc["task_cards"].as_object() {
                                        task_cards.clear();
                                        for (k, v) in cards {
                                            if let Some(val) = v.as_str() {
                                                task_cards.insert(k.clone(), val.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        adjusted_souls
                    } else {
                        souls
                    }
                }
                Err(_) => souls,
            }
        };

        // ── Phase: done ──
        send(serde_json::json!({
            "phase": "done",
            "response": {
                "entry_type": entry_type,
                "matched_souls": final_souls.iter().map(|s| serde_json::json!({
                    "name": s.name, "field": s.field, "ismism_code": s.ismism_code, "rationale": s.rationale,
                })).collect::<Vec<_>>(),
                "recommended_mode": mode,
                "review": { "verdict": final_verdict, "checks": final_checks, "notes": final_notes, "reviewer": reviewer_name },
                "task_cards": task_cards,
            }
        }).to_string());
    });

    Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

fn spawn_follow_up_agent(
    gateway: Arc<ai_gateway::GatewayRegistry>,
    banner_lord: SoulProfile,
    question: String,
    history: Vec<String>,
    attachments: Vec<AttachmentContent>,
    collector: Arc<crate::collector::SoulCollector>,
    ws: possession::WsSessionManager,
    session_id: String,
    archive: Arc<archive::ArchiveSystem>,
    summoned_soul: Option<(SoulProfile, String)>, // (profile, summon_reason)
    search_enabled: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let provider = gateway.pick_provider()
            .unwrap_or(foundation::Provider::Claude);

        let is_soul_summon = summoned_soul.is_some();
        let soul_name = if let Some((ref profile, _)) = summoned_soul {
            profile.name.clone()
        } else {
            banner_lord.name.clone()
        };

        // 议题搜索：用追问问题做 SearXNG，由前端开关控制。
        // 失败/超时静默 fallback 到无搜索结果。
        let search_results: Option<String> = if search_enabled && !question.trim().is_empty() {
            let query = question.clone();
            let trunc = query.len().min(160);
            let safe_idx = (0..=trunc).rev().find(|&i| query.is_char_boundary(i)).unwrap_or(0);
            tracing::info!("Follow-up: SearXNG quick search for: {}", &query[..safe_idx]);
            let fut = collector.search_topic_quick(&query, 3);
            match tokio::time::timeout(std::time::Duration::from_secs(12), fut).await {
                Ok(Ok(md)) if !md.trim().is_empty() => {
                    tracing::info!("Follow-up: SearXNG quick returned {} bytes", md.len());
                    Some(md)
                }
                Ok(Ok(_)) => {
                    tracing::info!("Follow-up: SearXNG quick returned empty");
                    None
                }
                Ok(Err(e)) => {
                    tracing::warn!("Follow-up: SearXNG quick failed: {}", e);
                    None
                }
                Err(_) => {
                    tracing::warn!("Follow-up: SearXNG quick timed out after 12s");
                    None
                }
            }
        } else {
            None
        };

        let search_section = match &search_results {
            Some(md) => format!("\n\n## 议题搜索结果（追问触发）\n{}\n", md),
            None => String::new(),
        };

        let attachment_section = if attachments.is_empty() {
            String::new()
        } else {
            let mut s = String::from("\n\n## 附件内容\n\n");
            for att in &attachments {
                s.push_str(&format!("### {}\n{}\n\n", att.filename, att.text));
            }
            s
        };

        let summon_reason_block = if let Some((ref _profile, ref reason)) = summoned_soul {
            format!("\n\n## 你被召唤的原因\n{}\n\n## 你的专属任务\n{}\n", reason, question)
        } else {
            String::new()
        };

        let _user_prompt = if history.is_empty() {
            format!(
                "## 新问题\n{}{}{}\n\n根据这个新问题，以你的立场和视角回应。你是{}，ismism坐标{}。{}",
                question, attachment_section, search_section,
                if is_soul_summon { &soul_name } else { &banner_lord.name },
                if is_soul_summon { summoned_soul.as_ref().map(|(p, _)| p.ismism_code.as_str()).unwrap_or("") } else { banner_lord.ismism_code.as_str() },
                summon_reason_block,
            )
        } else {
            format!(
                "## 历史对话\n{}\n\n## 新问题\n{}{}{}\n\n以上会话中，综合官判定需要补充你的视角。作为{}（ismism={}），请以你的立场和视角回应。如果上面有「议题搜索结果」，请将其作为事实弹药融入批判。{}",
                history.join("\n\n"), question, attachment_section, search_section,
                if is_soul_summon { &soul_name } else { &banner_lord.name },
                if is_soul_summon { summoned_soul.as_ref().map(|(p, _)| p.ismism_code.as_str()).unwrap_or("") } else { banner_lord.ismism_code.as_str() },
                summon_reason_block,
            )
        };

        let q_msg = foundation::Message {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            role: foundation::MessageRole::User,
            soul_name: None,
            content: question.clone(),
            seq: 990,
            created_at: chrono::Utc::now(),
        };
        let _ = archive.append_message(&q_msg).await;

        // Build the LLM prompt — use PromptBuilder for summoned souls to get full identity
        let (req, effective_soul_name) = if let Some((ref soul_profile, _)) = summoned_soul {
            let pb = ai_gateway::prompt::PromptBuilder::new();
            let tier = foundation::ModelTier::Pro;
            // Combine history + summon reason as facts context
            let facts = if history.is_empty() {
                summon_reason_block.trim().to_string()
            } else {
                format!("## 历史会话\n{}\n\n{}", history.join("\n\n"), summon_reason_block)
            };
            let ctx = ai_gateway::prompt::DynamicContext::new(question.clone())
                .with_facts_opt(if facts.is_empty() { None } else { Some(facts.as_str()) })
                .with_role(format!("你是被综合官点名补充视角的魂。你的任务是：{}", question));
            let prompt = pb.build_summon(soul_profile, &ctx, &tier, false);
            let config = CallConfig {
                temperature: 0.9,
                max_tokens: 32768,
                stream: true,
                model: None,
                reasoning_effort: Some(foundation::ReasoningEffort::Think),
                structured_output: None,
                thinking_enabled: None,
                tools: None,
                tool_choice: None,
            };
            (LLMRequest { provider, prompt, config }, soul_profile.name.clone())
        } else {
            // 非召唤追问也走 build_soul_identity，享受 FLP 框架
            let pb = ai_gateway::prompt::PromptBuilder::new();
            let tier = foundation::ModelTier::Pro;
            let facts = if history.is_empty() {
                String::new()
            } else {
                format!("## 历史会话\n{}", history.join("\n\n"))
            };
            let ctx = ai_gateway::prompt::DynamicContext::new(question.clone())
                .with_facts_opt(if facts.is_empty() { None } else { Some(facts.as_str()) })
                .with_era("2026年");
            let prompt = pb.build_summon(&banner_lord, &ctx, &tier, false);
            let config = CallConfig {
                temperature: 0.9,
                max_tokens: 32768,
                stream: true,
                model: None,
                reasoning_effort: Some(foundation::ReasoningEffort::Think),
                structured_output: None,
                thinking_enabled: None,
                tools: None,
                tool_choice: None,
            };
            (LLMRequest { provider, prompt, config }, soul_name.clone())
        };

        match gateway.call(&req) {
            Ok(rx) => {
                if is_soul_summon {
                    // 直接以魂身份流式输出 — 不走 synthesis
                    match possession::stream::stream_single_soul(rx, &session_id, &effective_soul_name, &ws).await {
                        output => {
                            let content = output.content.clone();
                            let notes = output.error.clone().unwrap_or_default();
                            let msg = foundation::Message {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: session_id.clone(),
                                role: foundation::MessageRole::Soul,
                                soul_name: Some(effective_soul_name.clone()),
                                content,
                                seq: 991,
                                created_at: chrono::Utc::now(),
                            };
                            let _ = archive.append_message(&msg).await;
                            let record = foundation::CallRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: session_id.clone(),
                                soul_name: effective_soul_name.clone(),
                                mode: foundation::PossessionMode::Conference,
                                task_summary: question.clone(),
                                effectiveness: if output.error.is_some() { foundation::Effectiveness::Invalid } else { foundation::Effectiveness::Effective },
                                notes: format!("[summoned-via-recommendation]{}", notes),
                                created_at: chrono::Utc::now(),
                                self_negation: None,
                                empty_chair: None,
                                user_feedback: None,
                                usage: output.usage,
                            };
                            let _ = archive.record_call(&record).await;
                            let _ = ws.broadcast_system(&session_id, &possession::WsEvent {
                                event_type: possession::WsEventType::SoulDone,
                                payload: String::new(),
                                reasoning_content: None,
                                soul_name: Some(soul_name.clone()),
                                seq: 0,
                            });
                        }
                    }
                } else {
                    match possession::stream::stream_synthesis(rx, &session_id, &ws).await {
                        Ok((content, usage)) => {
                            let msg = foundation::Message {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: session_id.clone(),
                                role: foundation::MessageRole::Synthesis,
                                soul_name: Some(banner_lord.name.clone()),
                                content,
                                seq: 991,
                                created_at: chrono::Utc::now(),
                            };
                            let _ = archive.append_message(&msg).await;
                            let record = foundation::CallRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: session_id.clone(),
                                soul_name: banner_lord.name,
                                mode: foundation::PossessionMode::Conference,
                                task_summary: question.clone(),
                                effectiveness: foundation::Effectiveness::Partial,
                                notes: "[follow-up]".to_string(),
                                created_at: chrono::Utc::now(),
                                self_negation: None,
                                empty_chair: None,
                                user_feedback: None,
                                usage,
                            };
                            let _ = archive.record_call(&record).await;
                        }
                        Err(e) => {
                            tracing::error!("Follow-up agent stream error: {}", e);
                            let _ = ws.broadcast_system(
                                &session_id,
                                &possession::WsEvent {
                                    event_type: possession::WsEventType::Error,
                                    payload: format!("流式响应错误: {}", e),
                                    reasoning_content: None,
                                    soul_name: None,
                                    seq: 0,
                                },
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Follow-up agent LLM call failed: {}", e);
                let _ = ws.broadcast_system(
                    &session_id,
                    &possession::WsEvent {
                        event_type: possession::WsEventType::Error,
                        payload: format!("启动 LLM 失败: {}", e),
                        reasoning_content: None,
                        soul_name: None,
                        seq: 0,
                    },
                );
            }
        }
    })
}

// ── Follow-up ──

const MAX_ATTACHMENTS: usize = 3;
const MAX_ATTACHMENT_SIZE: usize = 5 * 1024 * 1024;
const ALLOWED_ATTACHMENT_MIMES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif", "text/plain", "text/markdown", "application/pdf"];

/// 附件提取结果
struct AttachmentContent {
    filename: String,
    text: String,
}

async fn follow_up(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<ApiError>)> {
    tracing::info!("Received follow-up request for session: {}", session_id);

    let mut question = String::new();
    let mut attachments: Vec<AttachmentContent> = Vec::new();
    let mut requested_soul = String::new();
    let mut search_enabled = true; // default: enabled

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("question") => {
                if let Ok(text) = field.text().await {
                    question = text;
                }
            }
            Some("soul") => {
                if let Ok(text) = field.text().await {
                    requested_soul = text;
                }
            }
            Some("search") => {
                if let Ok(text) = field.text().await {
                    search_enabled = text == "true" || text == "1";
                }
            }
            Some("attachments") => {
                if let Some(filename) = field.file_name().map(String::from) {
                    if let Some(ct) = field.content_type().map(String::from) {
                        if !ALLOWED_ATTACHMENT_MIMES.contains(&ct.as_str()) {
                            tracing::warn!("Unsupported attachment type: {} for {}", ct, filename);
                            continue;
                        }
                    }
                    if let Ok(data) = field.bytes().await {
                        if data.len() > MAX_ATTACHMENT_SIZE {
                            tracing::warn!("Attachment {} exceeds 5MB, skipping", filename);
                            continue;
                        }
                        let text = if filename.to_lowercase().ends_with(".png")
                            || filename.to_lowercase().ends_with(".jpg")
                            || filename.to_lowercase().ends_with(".jpeg")
                            || filename.to_lowercase().ends_with(".webp")
                            || filename.to_lowercase().ends_with(".gif")
                        {
                            tokio::task::spawn_blocking(move || {
                                ocr::ocr_image(&data, "chi_sim+eng").ok().unwrap_or_default()
                            }).await.unwrap_or_default()
                        } else if filename.to_lowercase().ends_with(".pdf") {
                            // PDF 暂不支持，标记提示
                            format!("[PDF 文件 {}, 请提供文本内容]", filename)
                        } else {
                            // 文本文件直接读取
                            String::from_utf8_lossy(&data).to_string()
                        };
                        if !text.trim().is_empty() {
                            attachments.push(AttachmentContent { filename, text });
                        }
                        if attachments.len() >= MAX_ATTACHMENTS { break; }
                    }
                }
            }
            _ => {}
        }
    }

    if question.trim().is_empty() && attachments.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiError { error: "追问内容或附件至少提供一个".into() }),
        ));
    }

    let ws = state.engine.ws_manager().clone();
    let sid = session_id.clone();
    ws.create_session(&sid);

    // 优先用 observation 摘要替代原始魂输出作为追问上下文。
    // 6 个 observation ~600 字 vs 6 篇魂原文 ~20K 字，省两个数量级。
    // 自动蒸馏在 SessionComplete 前触发，但因异步可能尚未完成——轮询等待最多 8s。
    let mut history: Vec<String> = {
        let mut obs_result = state.archive.get_session_observations(&session_id).await;
        if obs_result.as_ref().map_or(true, |o| o.is_empty()) {
            tracing::info!("Observations not ready yet for {}, waiting up to 8s for distill...", session_id);
            for _ in 0..8 {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                obs_result = state.archive.get_session_observations(&session_id).await;
                if obs_result.as_ref().map_or(false, |o| !o.is_empty()) {
                    break;
                }
            }
        }
        match obs_result {
            Ok(obs) if !obs.is_empty() => {
                tracing::info!("Using {} observations as context for follow-up", obs.len());
                let mut ctx = vec![format!("## 历史会话核心观点（observation 摘要，{} 条）", obs.len())];
                for o in &obs {
                    let soul = o.soul_name.as_deref().unwrap_or("综合");
                    ctx.push(format!("- [{}] {}: {}", soul, o.title, o.content));
                }
                ctx
            }
            _ => {
                tracing::info!("Falling back to full message history for follow-up");
                match state.archive.get_session_detail(&session_id).await {
                    Ok(session) => {
                        session.messages.iter().map(|m| format!("[{:?}] {}: {}", m.role, m.soul_name.as_deref().unwrap_or("系统"), m.content)).collect()
                    }
                    Err(e) => {
                        tracing::warn!("Session not found: {}, proceeding with basic prompt", e);
                        vec![]
                    }
                }
            }
        }
    };
    // Truncate to recent context only — LLM context window is finite
    const MAX_HISTORY_ENTRIES: usize = 60;
    if history.len() > MAX_HISTORY_ENTRIES {
        let keep = history.len() - MAX_HISTORY_ENTRIES;
        history.drain(..keep);
    }

    // If frontend specified a soul, use it; otherwise fall back to default banner lord
    let responder = if !requested_soul.is_empty() {
        match state.registry.get_soul(&requested_soul) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Requested soul '{}' not found, falling back to default: {}", requested_soul, e);
                let reviewer_name = std::env::var("AIONUI_REVIEWER_SOUL").unwrap_or_else(|_| "未明子".to_string());
                state.registry.get_soul(&reviewer_name).map_err(|_| {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: format!("审查官 {} 不可用", reviewer_name) }))
                })?
            }
        }
    } else {
        let reviewer_name = std::env::var("AIONUI_REVIEWER_SOUL").unwrap_or_else(|_| "未明子".to_string());
        state.registry.get_soul(&reviewer_name).map_err(|_| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: format!("审查官 {} 不可用", reviewer_name) }))
        })?
    };

    let gateway = state.engine.gateway().clone();
    let archive = state.archive.clone();
    let collector = state.collector.clone();
    let attachment_count = attachments.len();

    // When a specific soul is requested, pass it as summoned_soul so it outputs
    // directly as a sub-agent rather than through the synthesis pipeline.
    let default_banner_lord = || -> Result<SoulProfile, _> {
        let reviewer_name = std::env::var("AIONUI_REVIEWER_SOUL").unwrap_or_else(|_| "未明子".to_string());
        state.registry.get_soul(&reviewer_name).map_err(|_| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: format!("审查官 {} 不可用", reviewer_name) }))
        })
    };

    let (banner_lord, summoned_soul) = if !requested_soul.is_empty() {
        let soul_profile = responder; // already looked up above
        let reason = format!("综合官在合议后发现需要补充你的视角。你的专属分析任务是：{}", question);
        (default_banner_lord()?, Some((soul_profile, reason)))
    } else {
        (responder, None)
    };

    let _ = spawn_follow_up_agent(
        gateway,
        banner_lord,
        question,
        history,
        attachments,
        collector,
        ws,
        sid,
        archive,
        summoned_soul,
        search_enabled,
    );

    tracing::info!("Follow-up sub-agent spawned with {} attachments", attachment_count);
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Status ──

#[derive(Debug, Serialize)]
struct PossessionStatusResponse { session_id: String, connected: bool }

async fn possession_status(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<PossessionStatusResponse> {
    Json(PossessionStatusResponse { session_id: session_id.clone(), connected: state.engine.ws_manager().has_session(&session_id) })
}

// ── OCR ──

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
const MAX_FILES: usize = 5;
const ALLOWED_MIMES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif"];

#[derive(Debug, Serialize)]
struct OcrResultItem {
    filename: String,
    text: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct OcrUploadResponse {
    results: Vec<OcrResultItem>,
}

async fn ocr_upload(
    State(_state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<OcrUploadResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    let mut file_data: Vec<(String, Vec<u8>)> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(filename) = field.file_name().map(String::from) else { continue; };
        let Some(content_type) = field.content_type().map(String::from) else { continue; };

        if !ALLOWED_MIMES.contains(&content_type.as_str()) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(ApiError { error: format!("不支持的文件类型: {}", content_type) }),
            ));
        }

        let Ok(data) = field.bytes().await else { continue; };
        if data.len() > MAX_FILE_SIZE {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(ApiError { error: format!("文件 {} 超过 10MB 限制", filename) }),
            ));
        }

        file_data.push((filename, data.to_vec()));
        if file_data.len() >= MAX_FILES { break; }
    }

    if file_data.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiError { error: "未收到有效文件".into() }),
        ));
    }

    let mut handles = Vec::new();
    for (filename, data) in file_data {
        let handle = tokio::task::spawn_blocking(move || {
            let text = ocr::ocr_image(&data, "chi_sim+eng");
            (filename, text)
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((filename, Ok(text))) => {
                let filtered = text.trim();
                results.push(OcrResultItem {
                    filename,
                    text: if filtered.is_empty() { None } else { Some(filtered.to_string()) },
                    error: None,
                });
            }
            Ok((filename, Err(e))) => {
                results.push(OcrResultItem { filename, text: None, error: Some(e) });
            }
            Err(e) => {
                results.push(OcrResultItem {
                    filename: "unknown".into(),
                    text: None,
                    error: Some(format!("spawn_blocking 错误: {}", e)),
                });
            }
        }
    }

    Ok(Json(OcrUploadResponse { results }))
}

// ─────────────────────────────────────────────
// 审查官入场审讯 — 合议前拦截
// ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct InterrogateRequest { task: String }

#[derive(Debug, Serialize)]
struct InterrogateResponse {
    gate_id: String,
    questions: Vec<InterrogationQuestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}


#[derive(Debug, Serialize)]
struct InterrogateVerdictResponse {
    passed: bool,
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    questions: Option<Vec<InterrogationQuestion>>,
    /// 审查官可能改写后的 finalized task（通过时）
    #[serde(skip_serializing_if = "Option::is_none")]
    refined_task: Option<String>,
}

/// POST /interrogate
/// 使用者提交议题 → 审查官生成 2-4 反问卡 → 返回 gate_id + questions
async fn start_interrogation(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InterrogateRequest>,
) -> Result<Json<InterrogateResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    let gateway = state.engine.gateway().clone();
    let pb = ai_gateway::prompt::PromptBuilder::new();
    let prompt = pb.build_interrogation_prompt(&body.task);

    let provider = pick_provider(&state)?;
    let req = LLMRequest {
        provider,
        prompt,
        config: CallConfig {
            temperature: 0.6,
            max_tokens: 4096, // 思考模型需要更多 token
            stream: false,
            ..Default::default()
        },
    };

    let mut rx = gateway.call(&req).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() }))
    })?;

    let (raw, first_err) = collect_llm_stream(&mut rx).await;
    if raw.is_empty() {
        let detail = first_err.unwrap_or_else(|| "LLM 返回空内容".to_string());
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: format!("审查官反问生成失败: {}", detail) })));
    }

    let json_str = extract_json_array(&raw).unwrap_or(&raw);
    tracing::debug!(raw_len = raw.len(), json_len = json_str.len(), "interrogate llm response");

    match serde_json::from_str::<Vec<InterrogationQuestion>>(json_str) {
        Ok(questions) => {
            let gate_id = uuid::Uuid::new_v4().to_string();
            let gate = InterrogationGate {
                task: body.task.clone(),
                questions: questions.clone(),
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            state.interrogation_gates.insert(gate_id.clone(), gate);

            if questions.is_empty() {
                // 审查官不可缴械——重试一次
                tracing::warn!("审查官返回空反问数组，重试…");
                let retry_raw = {
                    let retry_req = LLMRequest {
                        provider: provider.clone(),
                        prompt: pb.build_interrogation_prompt(&body.task),
                        config: CallConfig { temperature: 0.8, max_tokens: 4096, stream: false, ..Default::default() },
                    };
                    let mut retry_rx = gateway.call(&retry_req).map_err(|e| {
                        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() }))
                    })?;
                    let (retry_raw, _) = collect_llm_stream(&mut retry_rx).await;
                    extract_json_array(&retry_raw).unwrap_or(&retry_raw).to_string()
                };
                let retry_questions = serde_json::from_str::<Vec<InterrogationQuestion>>(&retry_raw)
                    .unwrap_or_else(|e| {
                        tracing::error!("审查官重试反问仍失败: {} (raw={})", e, retry_raw);
                        vec![
                            InterrogationQuestion { text: "你提问是为了什么——分析问题，还是推迟决定？".into(), required: true },
                            InterrogationQuestion { text: "在明天结束之前，你准备因为这个提问做什么具体动作？".into(), required: true },
                        ]
                    });
                if retry_questions.is_empty() {
                    // 最后的硬编码底线——审查官可被罢免，但门不能空
                    return Err((
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiError { error: "审查官未能生成反问，请稍后重试".into() }),
                    ));
                }
                Ok(Json(InterrogateResponse {
                    gate_id,
                    questions: retry_questions,
                    message: None,
                }))
            } else {
                Ok(Json(InterrogateResponse {
                    gate_id,
                    questions,
                    message: None,
                }))
            }
        }
        Err(e) => {
            let preview: String = raw.chars().take(200).collect();
            tracing::error!("Failed to parse interrogation questions: {} (raw={})", e, raw);
            tracing::error!("Failed to parse interrogation questions: {} (json_str={})", e, json_str);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError { error: format!("审查官反问解析失败: {} | raw={}", e, preview) }),
            ))
        }
    }
}

/// POST /interrogate/:gate_id/respond
/// 使用者提交对反问的回答 → 审查官裁决是否通过
async fn submit_interrogation(
    State(state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<InterrogateVerdictResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    tracing::info!("submit_interrogation called with gate_id={}", gate_id);

    // 读取 gate，提取 task 和 questions
    let (task, questions) = {
        let gate = state.interrogation_gates
            .get(&gate_id)
            .ok_or_else(|| {
                (axum::http::StatusCode::NOT_FOUND, Json(ApiError { error: "审讯门未找到，可能已过期".into() }))
            })?;
        (gate.task.clone(), gate.questions.clone())
    };

    // 配对所有回答与反问
    let answers_arr = body.get("answers").and_then(|a| a.as_array()).cloned().unwrap_or_default();
    let mut qa_pairs: Vec<(String, String)> = Vec::new();
    for ans in &answers_arr {
        let idx = ans.get("question_index").and_then(|v| v.as_u64()).unwrap_or(usize::MAX as u64) as usize;
        let text = ans.get("answer").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(q) = questions.get(idx) {
            qa_pairs.push((q.text.clone(), text.to_string()));
        }
    }

    state.interrogation_gates.remove(&gate_id);

    // 调 LLM 整合议题：把使用者的回答融进原始议题
    let refined_task = if !qa_pairs.is_empty() {
        let provider = match pick_provider(&state) {
            Ok(p) => p,
            Err(_) => {
                return Ok(Json(InterrogateVerdictResponse {
                    passed: true,
                    reason: "已收到回答。".into(),
                    questions: None,
                    refined_task: None,
                }))
            }
        };
        let pb = ai_gateway::prompt::PromptBuilder::new();
        let prompt = pb.build_task_refinement(&task, &qa_pairs);
        let req = LLMRequest {
            provider,
            prompt,
            config: CallConfig {
                temperature: 0.3,
                max_tokens: 1024,
                stream: false,
                thinking_enabled: Some(false),
                ..Default::default()
            },
        };

        match state.engine.gateway().call(&req) {
            Ok(mut rx) => {
                let (raw, _) = collect_llm_stream(&mut rx).await;
                let refined = raw.trim().to_string();
                if refined.is_empty() { None } else { Some(refined) }
            }
            Err(e) => {
                tracing::warn!("Task refinement LLM call failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    Ok(Json(InterrogateVerdictResponse {
        passed: true,
        reason: if refined_task.is_some() { "议题已整合。".into() } else { "已收到回答。".into() },
        questions: None,
        refined_task,
    }))
}
