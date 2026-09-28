use majstack_core::{MajstackError, Result, WorkType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    #[serde(rename = "type")]
    pub kind: String,
    pub instructions: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Value>,
}

impl Question {
    pub fn choice(instructions: impl Into<String>, criteria: BTreeMap<String, Value>) -> Self {
        Question {
            kind: "choice".into(),
            instructions: Value::String(instructions.into()),
            criteria: Some(serde_json::to_value(criteria).unwrap_or(Value::Null)),
        }
    }

    pub fn score(instructions: impl Into<String>, levels: Vec<String>) -> Self {
        Question {
            kind: "score".into(),
            instructions: Value::String(instructions.into()),
            criteria: Some(serde_json::to_value(levels).unwrap_or(Value::Null)),
        }
    }

    pub fn noul(instructions: impl Into<String>) -> Self {
        Question {
            kind: "noul".into(),
            instructions: Value::String(instructions.into()),
            criteria: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub choice: Option<String>,
    #[serde(default)]
    pub probabilities: Option<BTreeMap<String, f64>>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub legend: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub noul: Option<f64>,
}

impl Answer {
    pub fn as_choice(&self) -> Option<(&str, f64)> {
        self.choice
            .as_deref()
            .map(|choice| (choice, self.confidence.unwrap_or(0.0)))
    }

    pub fn as_probability(&self) -> Option<f64> {
        self.noul
    }

    pub fn as_score(&self) -> Option<(f64, f64)> {
        self.score
            .map(|score| (score, self.confidence.unwrap_or(0.0)))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EvaluationResponse {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub answers: BTreeMap<String, Answer>,
    #[serde(default)]
    pub usage: Usage,
}

pub trait SystemOne: Send + Sync {
    fn ask(
        &self,
        state: Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<EvaluationResponse>;
}

pub struct JevClient {
    base_url: String,
    model: String,
    api_key: String,
}

impl JevClient {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        JevClient {
            base_url: base_url.into(),
            model: model.into(),
            api_key: api_key.into(),
        }
    }

    pub fn from_env(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key_env: &str,
    ) -> Result<Self> {
        let api_key = std::env::var(api_key_env).map_err(|_| {
            MajstackError::Config(format!("environment variable {api_key_env} is not set"))
        })?;
        Ok(JevClient::new(base_url, model, api_key))
    }
}

impl SystemOne for JevClient {
    fn ask(
        &self,
        state: Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<EvaluationResponse> {
        let body = json!({
            "state": state,
            "model": self.model,
            "questions": questions,
        });
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let response = client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let status = response.status();
        let text = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(MajstackError::Network(format!(
                "TypeSafe API {status}: {text}"
            )));
        }
        serde_json::from_str(&text)
            .map_err(|error| MajstackError::Provider(format!("invalid TypeSafe response: {error}")))
    }
}

pub const WORK_TYPE_OPTIONS: &[&str] = &[
    "feature",
    "bug_fix",
    "investigation",
    "refactoring",
    "performance",
    "prototype",
    "architecture",
    "design",
    "visual_parity",
    "runtime_forensics",
    "trace_forensics",
    "security",
    "documentation",
    "release",
    "qa",
    "migration",
    "dependency_upgrade",
    "incident_repair",
    "repository_understanding",
    "technical_writing",
    "skill_authoring",
    "evaluation",
    "cleanup",
];

pub const WORK_TYPE_RUBRICS: &[&str] = &[
    "new or changed behavior",
    "a defect or regression to reproduce and fix",
    "a read-only question about how or why something works",
    "a behavior-preserving structural change",
    "measured slowness to improve",
    "a throwaway sketch to make a decision",
    "system-level structure and module boundaries",
    "visual or interaction design work",
    "pixel-exact UI equivalence",
    "a live runtime symptom such as a leak or spin",
    "a captured profiling artifact to diagnose",
    "vulnerabilities, auth, or an audit",
    "docs, readme, or changelog",
    "shipping, deploying, or publishing",
    "end-to-end or browser testing",
    "porting or migrating code or data",
    "upgrading a dependency",
    "an outage or production incident",
    "understanding an unfamiliar codebase",
    "prose or explanatory writing",
    "writing or editing an agent skill",
    "measuring agent or prompt behavior",
    "removing dead code or tidying",
];

pub fn classify_work_type(
    client: &dyn SystemOne,
    request: &str,
    threshold: f64,
) -> Result<(WorkType, f64)> {
    let criteria: BTreeMap<String, Value> = WORK_TYPE_OPTIONS
        .iter()
        .zip(WORK_TYPE_RUBRICS.iter())
        .map(|(option, rubric)| (option.to_string(), Value::String(rubric.to_string())))
        .collect();
    let mut questions = BTreeMap::new();
    questions.insert(
        "work_type".to_string(),
        Question::choice(
            "Classify this software engineering request into exactly one work type.",
            criteria,
        ),
    );
    let response = client.ask(json!({ "request": request }), questions)?;
    let answer = response
        .answers
        .get("work_type")
        .ok_or_else(|| MajstackError::Provider("no work_type answer".into()))?;
    let (choice, confidence) = answer
        .as_choice()
        .ok_or_else(|| MajstackError::Provider("work_type was not a choice".into()))?;
    let work_type = WorkType::parse(choice).unwrap_or(WorkType::Unknown);
    let resolved = if confidence < threshold {
        WorkType::Unknown
    } else {
        work_type
    };
    Ok((resolved, confidence))
}

pub fn needs_clarification(client: &dyn SystemOne, request: &str) -> Result<(bool, f64)> {
    let mut questions = BTreeMap::new();
    questions.insert(
        "ambiguous".to_string(),
        Question::noul(
            "Is this software request too ambiguous to act on without asking the user a clarifying question first?",
        ),
    );
    let response = client.ask(json!({ "request": request }), questions)?;
    let probability = response
        .answers
        .get("ambiguous")
        .and_then(Answer::as_probability)
        .unwrap_or(0.0);
    Ok((probability >= 0.5, probability))
}

pub fn verify_acceptance(
    client: &dyn SystemOne,
    criteria: &str,
    diff: &str,
) -> Result<(bool, f64)> {
    let mut questions = BTreeMap::new();
    questions.insert(
        "satisfied".to_string(),
        Question::noul(format!(
            "Does the change satisfy the acceptance criteria? Criteria: {criteria}"
        )),
    );
    let response = client.ask(json!({ "diff": diff }), questions)?;
    let probability = response
        .answers
        .get("satisfied")
        .and_then(Answer::as_probability)
        .unwrap_or(0.0);
    Ok((probability >= 0.5, probability))
}

#[derive(Debug, Clone, PartialEq)]
pub enum Gated<T> {
    Act(T),
    Escalate(T),
}

pub fn gate<T>(value: T, confidence: f64, threshold: f64) -> Gated<T> {
    if confidence >= threshold {
        Gated::Act(value)
    } else {
        Gated::Escalate(value)
    }
}

pub struct OpenAiSystemOne {
    url: String,
    model: String,
    api_key: String,
}

impl OpenAiSystemOne {
    pub fn new(base_url: &str, model: impl Into<String>, api_key: impl Into<String>) -> Self {
        let base = base_url.trim_end_matches('/');
        OpenAiSystemOne {
            url: format!("{base}/chat/completions"),
            model: model.into(),
            api_key: api_key.into(),
        }
    }
}

const DECISION_SYSTEM_PROMPT: &str = "You are a typed decision engine for software automation. Answer each question about the supplied state. Return ONLY JSON of the form {\"answers\":{\"<question id>\": <answer>}}. For a choice question include \"choice\", \"probabilities\", and \"confidence\". For a noul question include \"noul\" between 0 and 1. For a score question include \"score\", \"legend\", and \"confidence\". Never add prose.";

pub fn parse_answer_json(content: &str) -> Result<EvaluationResponse> {
    let trimmed = content.trim();
    let cleaned = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let value: Value = serde_json::from_str(cleaned)
        .map_err(|error| MajstackError::Provider(format!("invalid decision JSON: {error}")))?;
    if value.get("answers").is_some() {
        serde_json::from_value(value).map_err(|error| MajstackError::Provider(error.to_string()))
    } else {
        let answers: BTreeMap<String, Answer> = serde_json::from_value(value)
            .map_err(|error| MajstackError::Provider(error.to_string()))?;
        Ok(EvaluationResponse {
            model: String::new(),
            answers,
            usage: Usage::default(),
        })
    }
}

impl SystemOne for OpenAiSystemOne {
    fn ask(
        &self,
        state: Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<EvaluationResponse> {
        let questions_json = serde_json::to_string(&questions)?;
        let state_json = serde_json::to_string(&state)?;
        let body = json!({
            "model": self.model,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": DECISION_SYSTEM_PROMPT },
                { "role": "user", "content": format!("STATE:\n{state_json}\n\nQUESTIONS:\n{questions_json}") }
            ]
        });
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let mut request = client.post(&self.url).json(&body);
        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }
        let response = request
            .send()
            .map_err(|error| MajstackError::Network(error.to_string()))?;
        let status = response.status();
        let text = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(MajstackError::Network(format!(
                "decision endpoint {status}: {text}"
            )));
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| MajstackError::Provider(error.to_string()))?;
        let content = value
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                MajstackError::Provider("decision endpoint returned no content".into())
            })?;
        parse_answer_json(content)
    }
}

fn optional_env(env: &str) -> String {
    std::env::var(env).unwrap_or_default()
}

pub fn build_client(config: &majstack_config::TypedConfig) -> Result<Box<dyn SystemOne>> {
    match config.provider.as_str() {
        "typesafe" | "jev" => Ok(Box::new(JevClient::new(
            &config.base_url,
            &config.model,
            optional_env(&config.api_key_env),
        ))),
        "openai" | "compatible" | "local" => Ok(Box::new(OpenAiSystemOne::new(
            &config.base_url,
            &config.model,
            optional_env(&config.api_key_env),
        ))),
        other => Err(MajstackError::Config(format!(
            "unknown typed provider '{other}' (expected typesafe or openai)"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalCase {
    pub state: Value,
    pub question: String,
    pub options: Vec<String>,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSuite {
    pub name: String,
    #[serde(default)]
    pub instructions: String,
    pub cases: Vec<EvalCase>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalReport {
    pub name: String,
    pub total: usize,
    pub correct: usize,
    pub accurate_at_threshold: usize,
    pub below_threshold: usize,
    pub mean_confidence: f64,
    pub failures: Vec<String>,
}

impl EvalReport {
    pub fn accuracy(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.correct as f64 / self.total as f64
        }
    }
}

pub fn load_suite(text: &str) -> Result<EvalSuite> {
    toml::from_str(text)
        .map_err(|error| MajstackError::Config(format!("invalid eval suite: {error}")))
}

pub fn run_eval(client: &dyn SystemOne, suite: &EvalSuite, threshold: f64) -> Result<EvalReport> {
    let mut report = EvalReport {
        name: suite.name.clone(),
        ..EvalReport::default()
    };
    for (index, case) in suite.cases.iter().enumerate() {
        let criteria: BTreeMap<String, Value> = case
            .options
            .iter()
            .map(|option| (option.clone(), Value::Null))
            .collect();
        let mut questions = BTreeMap::new();
        questions.insert(
            "answer".to_string(),
            Question::choice(case.question.clone(), criteria),
        );
        let response = client.ask(case.state.clone(), questions)?;
        let answer = response
            .answers
            .get("answer")
            .and_then(Answer::as_choice)
            .ok_or_else(|| MajstackError::Provider(format!("case {index} returned no choice")))?;
        let (choice, confidence) = answer;
        report.total += 1;
        report.mean_confidence += confidence;
        if choice == case.expected {
            report.correct += 1;
            if confidence >= threshold {
                report.accurate_at_threshold += 1;
            }
        } else {
            report.failures.push(format!(
                "case {index}: expected '{}' got '{choice}' (confidence {confidence:.2})",
                case.expected
            ));
        }
        if confidence < threshold {
            report.below_threshold += 1;
        }
    }
    if report.total > 0 {
        report.mean_confidence /= report.total as f64;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn sample_response() -> &'static str {
        r#"{
          "model": "jev-1.13.0",
          "answers": {
            "department": { "type": "choice", "choice": "technical", "confidence": 0.81,
              "probabilities": { "technical": 0.85, "billing": 0.15, "sales": 0.0 } },
            "frustration": { "type": "score", "score": 1.05, "confidence": 0.92,
              "legend": { "0": "Calm", "1": "Frustrated", "2": "Very angry" },
              "probabilities": { "0": 0.0, "1": 0.95, "2": 0.05 } },
            "is_urgent": { "type": "noul", "noul": 0.95 }
          },
          "usage": { "input_tokens": 392, "output_tokens": 65 }
        }"#
    }

    #[test]
    fn parses_typed_answers() {
        let response: EvaluationResponse = serde_json::from_str(sample_response()).unwrap();
        assert_eq!(
            response.answers["department"].as_choice(),
            Some(("technical", 0.81))
        );
        assert_eq!(response.answers["is_urgent"].as_probability(), Some(0.95));
        assert_eq!(
            response.answers["frustration"].as_score(),
            Some((1.05, 0.92))
        );
        assert_eq!(response.usage.input_tokens, 392);
    }

    #[test]
    fn serializes_questions_with_correct_tags() {
        let question = Question::noul("Is it urgent?");
        let value = serde_json::to_value(&question).unwrap();
        assert_eq!(value["type"], "noul");
        assert_eq!(value["instructions"], "Is it urgent?");
        assert!(value.get("criteria").is_none());

        let mut criteria = BTreeMap::new();
        criteria.insert("a".to_string(), Value::String("first".into()));
        let choice = serde_json::to_value(Question::choice("pick", criteria)).unwrap();
        assert_eq!(choice["type"], "choice");
        assert_eq!(choice["criteria"]["a"], "first");
    }

    struct FakeSystemOne {
        last_questions: Mutex<BTreeMap<String, Question>>,
        answer: Answer,
    }

    impl SystemOne for FakeSystemOne {
        fn ask(
            &self,
            _state: Value,
            questions: BTreeMap<String, Question>,
        ) -> Result<EvaluationResponse> {
            *self.last_questions.lock().unwrap() = questions;
            let mut answers = BTreeMap::new();
            answers.insert("work_type".to_string(), self.answer.clone());
            Ok(EvaluationResponse {
                model: "fake".into(),
                answers,
                usage: Usage::default(),
            })
        }
    }

    fn choice_answer(choice: &str, confidence: f64) -> Answer {
        Answer {
            kind: "choice".into(),
            choice: Some(choice.into()),
            probabilities: None,
            confidence: Some(confidence),
            score: None,
            legend: None,
            noul: None,
        }
    }

    #[test]
    fn classifies_work_type_and_applies_confidence() {
        let fake = FakeSystemOne {
            last_questions: Mutex::new(BTreeMap::new()),
            answer: choice_answer("bug_fix", 0.9),
        };
        let (work_type, confidence) = classify_work_type(&fake, "fix the crash", 0.6).unwrap();
        assert_eq!(work_type, WorkType::BugFix);
        assert!(confidence >= 0.6);
        assert!(fake
            .last_questions
            .lock()
            .unwrap()
            .contains_key("work_type"));
    }

    #[test]
    fn low_confidence_falls_back_to_unknown() {
        let fake = FakeSystemOne {
            last_questions: Mutex::new(BTreeMap::new()),
            answer: choice_answer("security", 0.2),
        };
        let (work_type, _) = classify_work_type(&fake, "do the thing", 0.6).unwrap();
        assert_eq!(work_type, WorkType::Unknown);
    }

    #[test]
    fn confidence_gate_routes() {
        assert_eq!(gate("x", 0.9, 0.6), Gated::Act("x"));
        assert_eq!(gate("x", 0.3, 0.6), Gated::Escalate("x"));
    }

    #[test]
    fn parses_openai_style_answer_json() {
        let content = "```json\n{\"answers\":{\"work_type\":{\"type\":\"choice\",\"choice\":\"performance\",\"confidence\":0.77}}}\n```";
        let response = parse_answer_json(content).unwrap();
        assert_eq!(
            response.answers["work_type"].as_choice(),
            Some(("performance", 0.77))
        );
    }

    #[test]
    fn build_client_selects_provider() {
        let config = majstack_config::TypedConfig {
            provider: "openai".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "qwen2.5:7b".into(),
            ..majstack_config::TypedConfig::default()
        };
        assert!(build_client(&config).is_ok());
        let bad = majstack_config::TypedConfig {
            provider: "nope".into(),
            ..majstack_config::TypedConfig::default()
        };
        assert!(build_client(&bad).is_err());
    }
}
