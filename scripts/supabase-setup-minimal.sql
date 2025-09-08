-- Minimal Supabase Setup for KotaDB API Key Management
-- This focuses only on API key authentication, not document storage

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Drop existing tables if they exist (for clean setup)
DROP TABLE IF EXISTS usage_metrics CASCADE;
DROP TABLE IF EXISTS api_keys CASCADE;

-- Create API Keys table (for authentication only)
CREATE TABLE api_keys (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    permissions JSONB DEFAULT '{"read": true, "write": false}'::jsonb,
    rate_limit INTEGER DEFAULT 60,
    monthly_quota INTEGER DEFAULT 1000000,
    usage_count INTEGER DEFAULT 0,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create Usage Metrics table (for tracking API usage)
CREATE TABLE usage_metrics (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    api_key_id UUID REFERENCES api_keys(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER,
    response_time_ms INTEGER,
    tokens_used INTEGER DEFAULT 0,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_expires_at ON api_keys(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_usage_metrics_api_key ON usage_metrics(api_key_id, created_at DESC);
CREATE INDEX idx_usage_metrics_created_at ON usage_metrics(created_at DESC);

-- Enable Row Level Security
ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE usage_metrics ENABLE ROW LEVEL SECURITY;

-- API Keys RLS Policies
CREATE POLICY "Users can view their own API keys"
    ON api_keys FOR SELECT
    USING (auth.uid() = user_id);

CREATE POLICY "Users can create their own API keys"
    ON api_keys FOR INSERT
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can update their own API keys"
    ON api_keys FOR UPDATE
    USING (auth.uid() = user_id)
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "Users can delete their own API keys"
    ON api_keys FOR DELETE
    USING (auth.uid() = user_id);

-- Usage Metrics RLS Policies (read-only for users)
CREATE POLICY "Users can view metrics for their API keys"
    ON usage_metrics FOR SELECT
    USING (
        api_key_id IN (
            SELECT id FROM api_keys WHERE user_id = auth.uid()
        )
    );

-- Service role can do everything (for backend API)
CREATE POLICY "Service role has full access to api_keys"
    ON api_keys FOR ALL
    USING (auth.role() = 'service_role');

CREATE POLICY "Service role has full access to usage_metrics"
    ON usage_metrics FOR ALL
    USING (auth.role() = 'service_role');

-- Create helper functions

-- Function to check rate limits
CREATE OR REPLACE FUNCTION check_rate_limit(p_api_key_id UUID)
RETURNS BOOLEAN AS $$
DECLARE
    v_count INTEGER;
    v_rate_limit INTEGER;
BEGIN
    -- Get the rate limit for this API key
    SELECT rate_limit INTO v_rate_limit
    FROM api_keys
    WHERE id = p_api_key_id;
    
    -- Count requests in the last minute
    SELECT COUNT(*) INTO v_count
    FROM usage_metrics
    WHERE api_key_id = p_api_key_id
      AND created_at > NOW() - INTERVAL '1 minute';
    
    RETURN v_count < v_rate_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to check monthly quota
CREATE OR REPLACE FUNCTION check_monthly_quota(p_api_key_id UUID)
RETURNS BOOLEAN AS $$
DECLARE
    v_usage_count INTEGER;
    v_monthly_quota INTEGER;
BEGIN
    SELECT usage_count, monthly_quota 
    INTO v_usage_count, v_monthly_quota
    FROM api_keys
    WHERE id = p_api_key_id;
    
    RETURN v_usage_count < v_monthly_quota;
END;
$$ LANGUAGE plpgsql;

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_api_keys_updated_at
    BEFORE UPDATE ON api_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Create a view for API key statistics
CREATE OR REPLACE VIEW api_key_stats AS
SELECT 
    ak.id,
    ak.name,
    ak.user_id,
    ak.usage_count,
    ak.monthly_quota,
    ak.rate_limit,
    COUNT(um.id) AS requests_last_hour,
    AVG(um.response_time_ms) AS avg_response_time_ms,
    MAX(um.created_at) AS last_request_at
FROM api_keys ak
LEFT JOIN usage_metrics um ON ak.id = um.api_key_id 
    AND um.created_at > NOW() - INTERVAL '1 hour'
GROUP BY ak.id, ak.name, ak.user_id, ak.usage_count, ak.monthly_quota, ak.rate_limit;

-- Grant permissions on the view
GRANT SELECT ON api_key_stats TO authenticated;

-- Success message
DO $$
BEGIN
    RAISE NOTICE 'Minimal Supabase setup completed successfully!';
    RAISE NOTICE 'Tables created: api_keys, usage_metrics';
    RAISE NOTICE 'RLS policies applied for API key management';
    RAISE NOTICE 'Documents will be stored in KotaDB local storage, not Supabase';
END $$;