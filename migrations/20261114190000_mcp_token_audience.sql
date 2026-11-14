ALTER TABLE api_tokens DROP CONSTRAINT api_tokens_audience_check;
ALTER TABLE api_tokens ADD CONSTRAINT api_tokens_audience_check CHECK (audience IN ('api', 'mcp'));
