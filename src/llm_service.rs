use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_json;
use crate::models::{SearchIntent, EvaluatedResult, SearchStrategy};
use crate::pirate_bay_scraper::TorrentResult;
use crate::prompts::{build_parse_prompt, build_evaluation_prompt, build_query_generation_prompt};

pub struct LlmService {
    ollama: Ollama,
    model: String,
    temperature: f32,
}

impl LlmService {
    pub fn new(model: String) -> Result<Self> {
        let ollama = Ollama::default();
        Ok(Self {
            ollama,
            model,
            temperature: 0.3, // Low temperature for consistent parsing
        })
    }

    pub async fn parse_query(&self, query: &str) -> Result<SearchIntent> {
        let prompt = build_parse_prompt(query);
        let response = self.generate(&prompt).await?;
        self.parse_json_response(&response)
    }

    pub async fn evaluate_results(
        &self, 
        intent: &SearchIntent, 
        results: Vec<TorrentResult>
    ) -> Result<Vec<EvaluatedResult>> {
        let prompt = build_evaluation_prompt(intent, &results);
        let response = self.generate(&prompt).await?;
        self.parse_evaluation_response(&response, results)
    }

    pub async fn generate_search_queries(&self, intent: &SearchIntent) -> Result<SearchStrategy> {
        let prompt = build_query_generation_prompt(intent);
        let response = self.generate(&prompt).await?;
        self.parse_json_response(&response)
    }

    async fn generate(&self, prompt: &str) -> Result<String> {
        let request = GenerationRequest::new(self.model.clone(), prompt.to_string());
        
        let response = self.ollama.generate(request).await?;
        Ok(response.response)
    }

    pub async fn health_check(&self) -> Result<bool> {
        match self.ollama.list_local_models().await {
            Ok(_) => Ok(true),
            Err(_) => Err(anyhow::anyhow!("Ollama is not running. Start with: ollama serve"))
        }
    }

    pub async fn ensure_model(&self) -> Result<()> {
        let models = self.ollama.list_local_models().await?;
        if !models.iter().any(|m| m.name == self.model) {
            return Err(anyhow::anyhow!("Model {} not found. Pull with: ollama pull {}", 
                                       self.model, self.model));
        }
        Ok(())
    }

    fn parse_json_response<T: DeserializeOwned>(&self, response: &str) -> Result<T> {
        // Try to extract JSON from response (LLM might add explanation)
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))
    }

    fn parse_evaluation_response(&self, response: &str, results: Vec<TorrentResult>) -> Result<Vec<EvaluatedResult>> {
        // Extract JSON array from response
        let json_start = response.find('[').unwrap_or(0);
        let json_end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        let evaluations: Vec<serde_json::Value> = serde_json::from_str(json_str)?;
        
        let mut evaluated_results = Vec::new();
        for (i, eval) in evaluations.iter().enumerate() {
            if let Some(torrent) = results.get(i) {
                let evaluated = EvaluatedResult {
                    torrent: torrent.clone(),
                    relevance_score: eval["relevance_score"].as_f64().unwrap_or(0.0) as f32,
                    confidence: eval["confidence"].as_f64().unwrap_or(0.0) as f32,
                    match_reasons: eval["match_reasons"]
                        .as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect())
                        .unwrap_or_default(),
                    warnings: eval["warnings"]
                        .as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect())
                        .unwrap_or_default(),
                    quality_score: eval["quality_score"].as_f64().unwrap_or(0.0) as f32,
                    completeness_score: eval["completeness_score"].as_f64().unwrap_or(0.0) as f32,
                };
                evaluated_results.push(evaluated);
            }
        }
        
        Ok(evaluated_results)
    }
}