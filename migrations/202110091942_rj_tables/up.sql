CREATE TABLE rj_admin_settings (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	tts_service_url TEXT,
	tts_text_param_key TEXT,
	tts_enable_ssml INTEGER,
	UNIQUE(id) ON CONFLICT REPLACE
);

CREATE TABLE rj_user_settings (
	id INTEGER PRIMARY KEY NOT NULL CHECK(id = 0),
	scripts TEXT,
	enable_by_default INTEGER,
	tts_people TEXT,
	UNIQUE(id) ON CONFLICT REPLACE
);

INSERT INTO rj_admin_settings (id, tts_enable_ssml) VALUES (0, 0);
INSERT INTO rj_user_settings (id, tts_people) VALUES (0, '[]');

