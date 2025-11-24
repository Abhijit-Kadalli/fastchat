.PHONY: help ollama lmstudio stop list-models

help:
	@echo "Available commands:"
	@echo "  make ollama      - Start Ollama backend"
	@echo "  make lmstudio    - Start LM Studio compatible backend (LocalAI)"
	@echo "  make stop        - Stop all backends"
	@echo "  make list-models - List models installed in Ollama"

ollama:
	@echo "Starting Ollama..."
	@cd backend && docker compose --profile ollama up -d

lmstudio:
	@echo "Starting LM Studio compatible backend..."
	@cd backend && docker compose --profile lmstudio up -d

stop:
	@echo "Stopping backends..."
	@cd backend && docker compose down

list-models:
	@echo "Listing Ollama models..."
	@docker exec fastchat_ollama ollama list
