CREATE SCHEMA IF NOT EXISTS snippets_extension;
CREATE SCHEMA IF NOT EXISTS write_right;
CREATE SCHEMA IF NOT EXISTS sync_flow;

-- Enable pgcrypto for UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ________________________________ Public ________________________________

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE NOT NULL,
    full_name TEXT,
    profile_picture_url TEXT,
    password_hash TEXT NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE user_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_id TEXT NOT NULL,
  refresh_token TEXT NOT NULL,
  user_agent TEXT,
  ip_address TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_used_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  revoked BOOLEAN NOT NULL DEFAULT FALSE,
  UNIQUE (user_id, device_id)
);

CREATE INDEX idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_token ON user_sessions(refresh_token);
CREATE UNIQUE INDEX uniq_user_sessions_user_device ON user_sessions(user_id, device_id);



CREATE TRIGGER trg_users_updated_at
  BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TYPE subscription_plan AS ENUM ('free', 'pro');
CREATE TYPE subscription_status AS ENUM ('active', 'canceled', 'incomplete', 'past_due', 'unpaid');

CREATE TABLE subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  plan subscription_plan NOT NULL DEFAULT 'free',
  stripe_customer_id TEXT,
  stripe_subscription_id TEXT,
  starts_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  ends_at TIMESTAMPTZ,
  status subscription_status NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ________________________________ VSC Snippet Extension ________________________________
CREATE TABLE snippets_extension.snippets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_id UUID NOT NULL REFERENCES users(id),
  title TEXT NOT NULL,
  description TEXT,
  code TEXT,
  language TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER trg_snippets_updated_at
  BEFORE UPDATE ON snippets_extension.snippets
  FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE INDEX idx_snippets_language ON snippets_extension.snippets(language);
CREATE INDEX idx_snippets_owner ON snippets_extension.snippets(owner_id);

CREATE TABLE snippets_extension.tags (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT UNIQUE NOT NULL
);

CREATE TABLE snippets_extension.snippet_tags (
  snippet_id UUID NOT NULL REFERENCES snippets_extension.snippets(id) ON DELETE CASCADE,
  tag_id UUID NOT NULL REFERENCES snippets_extension.tags(id) ON DELETE CASCADE,
  PRIMARY KEY (snippet_id, tag_id)
);

CREATE TABLE snippets_extension.snippet_stars (
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  snippet_id UUID NOT NULL REFERENCES snippets_extension.snippets(id) ON DELETE CASCADE,
  starred_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (user_id, snippet_id)
);

CREATE INDEX idx_snippet_stars_user ON snippets_extension.snippet_stars(user_id);
CREATE INDEX idx_snippet_stars_snippet ON snippets_extension.snippet_stars(snippet_id);


-- ________________________________ Write Right (Markdown WYSIWYG) ________________________________

-- ________________________________ Sync Flow (Notion/Trello) ________________________________
