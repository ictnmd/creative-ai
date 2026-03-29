-- ============================================================
-- Creative AI Studio - Initial PostgreSQL Schema
-- Migration: 001_initial_schema
-- ============================================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- ENUMS
-- ============================================================

CREATE TYPE auth_provider AS ENUM ('google', 'github', 'email');

CREATE TYPE subscription_tier AS ENUM ('free', 'basic', 'pro', 'unlimited');

CREATE TYPE user_role AS ENUM ('user', 'admin');

CREATE TYPE generation_status AS ENUM ('pending', 'processing', 'completed', 'failed');

CREATE TYPE visibility AS ENUM ('private', 'shared', 'public');

CREATE TYPE subscription_status AS ENUM ('active', 'cancelled', 'past_due');

CREATE TYPE transaction_type AS ENUM ('purchase', 'usage', 'refund', 'bonus');

-- ============================================================
-- TABLE: users
-- ============================================================

CREATE TABLE users (
    id            UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    username      VARCHAR(50) NOT NULL UNIQUE,
    email         VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255),
    name          VARCHAR(100),
    avatar_url    TEXT,
    auth_provider auth_provider NOT NULL DEFAULT 'email',
    subscription_tier subscription_tier NOT NULL DEFAULT 'free',
    role          user_role   NOT NULL DEFAULT 'user',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users (username);
CREATE INDEX idx_users_email ON users (email);

-- ============================================================
-- TABLE: refresh_tokens
-- ============================================================

CREATE TABLE refresh_tokens (
    id          UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  VARCHAR(255) NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens (user_id);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens (expires_at);

-- ============================================================
-- TABLE: user_profiles
-- ============================================================

CREATE TABLE user_profiles (
    user_id          UUID        NOT NULL PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    bio              TEXT,
    showcase_ids     UUID[]      NOT NULL DEFAULT '{}',
    is_public_profile BOOLEAN    NOT NULL DEFAULT TRUE
);

-- ============================================================
-- TABLE: follows
-- ============================================================

CREATE TABLE follows (
    follower_id  UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (follower_id, following_id),
    CONSTRAINT follows_no_self_follow CHECK (follower_id != following_id)
);

CREATE INDEX idx_follows_follower ON follows (follower_id);
CREATE INDEX idx_follows_following ON follows (following_id);

-- ============================================================
-- TABLE: account_deletions
-- ============================================================

CREATE TABLE account_deletions (
    user_id     UUID        NOT NULL PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    deleted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    purge_after TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 days')
);

-- ============================================================
-- TABLE: style_presets
-- ============================================================

CREATE TABLE style_presets (
    id              UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    prompt_suffix   TEXT        NOT NULL DEFAULT '',
    thumbnail_url   TEXT,
    is_public       BOOLEAN    NOT NULL DEFAULT FALSE,
    is_builtin      BOOLEAN    NOT NULL DEFAULT FALSE,
    creator_id      UUID        REFERENCES users(id) ON DELETE SET NULL,
    tags            TEXT[]      NOT NULL DEFAULT '{}',
    usage_count     INTEGER     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TABLE: generations
-- ============================================================

CREATE TABLE generations (
    id              UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    prompt          TEXT        NOT NULL,
    enhanced_prompt TEXT,
    provider        VARCHAR(50),
    model           VARCHAR(100),
    style_preset_id UUID        REFERENCES style_presets(id) ON DELETE SET NULL,
    reference_images TEXT[]     NOT NULL DEFAULT '{}',
    sketch_data     TEXT,
    output_urls     TEXT[]      NOT NULL DEFAULT '{}',
    status          generation_status NOT NULL DEFAULT 'pending',
    error_message   TEXT,
    credits_used    INTEGER     NOT NULL DEFAULT 0,
    quota_used      INTEGER     NOT NULL DEFAULT 0,
    visibility      visibility  NOT NULL DEFAULT 'private',
    view_count      INTEGER     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_generations_user_id ON generations (user_id);
CREATE INDEX idx_generations_status ON generations (status);
CREATE INDEX idx_generations_created_at ON generations (created_at);
CREATE INDEX idx_generations_provider ON generations (provider);

-- ============================================================
-- TABLE: subscriptions
-- ============================================================

CREATE TABLE subscriptions (
    id                    UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id               UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier                  subscription_tier NOT NULL,
    stripe_subscription_id VARCHAR(255),
    stripe_customer_id    VARCHAR(255),
    quota_monthly         INTEGER     NOT NULL DEFAULT 0,
    starts_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at            TIMESTAMPTZ NOT NULL,
    status                subscription_status NOT NULL DEFAULT 'active',
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_user_id ON subscriptions (user_id);
CREATE INDEX idx_subscriptions_status ON subscriptions (status);

-- ============================================================
-- TABLE: credit_transactions
-- ============================================================

CREATE TABLE credit_transactions (
    id                UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id           UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount            INTEGER     NOT NULL,
    transaction_type   transaction_type NOT NULL,
    description       TEXT,
    stripe_payment_id  VARCHAR(255),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_credit_transactions_user_id ON credit_transactions (user_id);
CREATE INDEX idx_credit_transactions_created_at ON credit_transactions (created_at);

-- ============================================================
-- TABLE: shared_generations
-- ============================================================

CREATE TABLE shared_generations (
    id            UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    generation_id UUID        NOT NULL UNIQUE REFERENCES generations(id) ON DELETE CASCADE,
    share_token   VARCHAR(64) NOT NULL UNIQUE,
    is_public     BOOLEAN    NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_shared_generations_share_token ON shared_generations (share_token);

-- ============================================================
-- TABLE: user_api_keys
-- ============================================================

CREATE TABLE user_api_keys (
    id           UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider     VARCHAR(50) NOT NULL,
    encrypted_key TEXT       NOT NULL,
    label        VARCHAR(100),
    is_active    BOOLEAN    NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TABLE: system_api_keys
-- ============================================================

CREATE TABLE system_api_keys (
    id           UUID        NOT NULL DEFAULT uuid_generate_v4() PRIMARY KEY,
    provider     VARCHAR(50) NOT NULL,
    encrypted_key TEXT       NOT NULL,
    is_active    BOOLEAN    NOT NULL DEFAULT TRUE,
    rate_limit   INTEGER     NOT NULL DEFAULT 60,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SEED DATA: Default Style Presets
-- ============================================================

INSERT INTO style_presets (name, description, prompt_suffix, thumbnail_url, is_public, is_builtin, tags) VALUES
(
    'Photorealistic',
    'Ultra-detailed, lifelike images with realistic lighting, textures, and depth of field.',
    ', ultra detailed, photorealistic, 8k, cinematic lighting, sharp focus, professional photography, high resolution',
    NULL,
    TRUE,
    TRUE,
    ARRAY['photorealistic', 'realistic', 'photo', 'cinematic', 'professional']
),
(
    'Anime',
    'Japanese anime and manga style with vibrant colors, expressive eyes, and dynamic compositions.',
    ', anime style, manga art, vibrant colors, detailed anime eyes, cel shading, japanese art style',
    NULL,
    TRUE,
    TRUE,
    ARRAY['anime', 'manga', 'japanese', 'cartoon', 'vibrant']
),
(
    'Oil Painting',
    'Classic oil painting aesthetic with rich textures, visible brushstrokes, and classical composition.',
    ', oil painting style, rich textures, visible brushstrokes, classical composition, museum quality, fine art',
    NULL,
    TRUE,
    TRUE,
    ARRAY['painting', 'oil', 'classical', 'artistic', 'traditional']
),
(
    'Watercolor',
    'Soft, flowing watercolor painting with gentle color gradients and delicate paper textures.',
    ', watercolor painting, soft colors, flowing washes, delicate paper texture, artistic, impressionistic',
    NULL,
    TRUE,
    TRUE,
    ARRAY['watercolor', 'soft', 'delicate', 'painting', 'artistic']
),
(
    'Digital Art',
    'Modern digital illustration with clean lines, vibrant gradients, and contemporary aesthetic.',
    ', digital art, digital illustration, clean lines, vibrant gradients, modern style, trending on artstation',
    NULL,
    TRUE,
    TRUE,
    ARRAY['digital', 'illustration', 'modern', 'gradient', 'contemporary']
),
(
    'Minimalist',
    'Clean, minimalist compositions with simple forms, limited color palette, and elegant negative space.',
    ', minimalist style, clean composition, simple forms, limited color palette, elegant, modern design',
    NULL,
    TRUE,
    TRUE,
    ARRAY['minimalist', 'minimal', 'clean', 'simple', 'modern']
),
(
    '3D Render',
    'High-quality 3D rendered imagery with realistic materials, lighting, and depth.',
    ', 3d render, octane render, cinema 4d, blender 3d, ray tracing, volumetric lighting, 8k, detailed textures',
    NULL,
    TRUE,
    TRUE,
    ARRAY['3d', 'render', '3d render', 'cgi', 'three-dimensional']
),
(
    'Sketch',
    'Hand-drawn sketch aesthetic with pencil lines, cross-hatching, and expressive linework.',
    ', hand drawn sketch, pencil sketch, cross hatching, expressive linework, paper texture, artistic drawing',
    NULL,
    TRUE,
    TRUE,
    ARRAY['sketch', 'drawing', 'pencil', 'hand-drawn', 'linework']
);

-- ============================================================
-- TRIGGER: Auto-update updated_at on users
-- ============================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
