--
-- PostgreSQL database dump
--

\restrict 7hRJ77ZVlQH3PV8UkbKzQUFfDZjO6WDFhuY3VRQC9HVCAs3MuTWslRwajoO8guN

-- Dumped from database version 18.3 (Debian 18.3-1.pgdg13+1)
-- Dumped by pg_dump version 18.3 (Debian 18.3-1.pgdg13+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pg_trgm; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pg_trgm WITH SCHEMA public;


--
-- Name: EXTENSION pg_trgm; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION pg_trgm IS 'text similarity measurement and index searching based on trigrams';


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


--
-- Name: audit_logs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.audit_logs (
    id uuid NOT NULL,
    user_id uuid,
    action character varying(50) NOT NULL,
    target_type character varying(20),
    target_id uuid,
    details jsonb,
    ip_address character varying(45),
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: auth_state; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.auth_state (
    id smallint NOT NULL,
    session_epoch bigint DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT chk_auth_state_singleton CHECK ((id = 1))
);


--
-- Name: email_verification_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.email_verification_tokens (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    token_hash character varying(128) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: images; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.images (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    filename character varying(255) NOT NULL,
    thumbnail character varying(255),
    size bigint NOT NULL,
    hash character varying(64) NOT NULL,
    format character varying(20),
    views integer DEFAULT 0,
    status character varying(20) DEFAULT 'active'::character varying,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: media_blobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.media_blobs (
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    storage_key character varying(255) NOT NULL,
    media_kind character varying(32) DEFAULT 'original'::character varying NOT NULL,
    content_hash character varying(64),
    ref_count bigint DEFAULT 0 NOT NULL,
    status character varying(32) DEFAULT 'ready'::character varying NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: password_reset_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.password_reset_tokens (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    token_hash character varying(128) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    used_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: revoked_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.revoked_tokens (
    token_hash character varying(64) NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: settings; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.settings (
    key character varying(50) NOT NULL,
    value text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: storage_cleanup_jobs; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.storage_cleanup_jobs (
    id uuid NOT NULL,
    file_key character varying(255) NOT NULL,
    storage_signature character(64) NOT NULL,
    storage_snapshot text NOT NULL,
    attempts bigint DEFAULT 0 NOT NULL,
    last_error text,
    next_attempt_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id uuid NOT NULL,
    email character varying(255) NOT NULL,
    email_verified_at timestamp with time zone,
    password_hash character varying(255) NOT NULL,
    role character varying(20) DEFAULT 'admin'::character varying,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    token_version bigint DEFAULT 0 NOT NULL
);


--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
1	initial schema	2026-04-07 13:07:49.576539+00	t	\\xe3d46ad709293dccd10547fadd6cde8ce8d4874417609b40088540b5c0e817ae5f1ef737d7853defcd7298ff4a2e7e93	42740898
2	drop unique filename index	2026-04-07 13:07:49.620467+00	t	\\x9c089c99b82b83f67e4a9fe2ff47b4483c1475ba7be56e320a2910efd939504c9a57b87f390151db31da31fa0ea79dc8	2157533
3	add auth runtime state	2026-04-07 13:07:49.623296+00	t	\\xf331d638badd2003158f875db2ec16a71a6b0e8721e5b605be725ee7c0f327eeaa32e65ccd2c2be3e4caa53c4cac516a	4260184
4	remove unused image metadata	2026-04-07 13:07:49.628176+00	t	\\x6929d35f692353eb93012002a4e4b54cac8d87f58afcb18248f908bf7a1a336e353a24cc834d52e3861790324062160a	6411918
5	add storage cleanup jobs	2026-04-07 13:07:49.635301+00	t	\\x3e89c45c232972ecd4dc3513b071546a12d66b4ad43d3e868010a0f67d416dba97340acec0cd8c1cae23cfd5b4e0c972	3637685
6	add media blobs	2026-04-07 13:07:49.63967+00	t	\\xc06bd823dd4898650002501957982f086a053581e95cf8df99821971a993cfdc94ce2fb63eaffdc24f7bfc09739ff125	3486393
7	reconcile media blobs schema	2026-04-07 13:07:49.643857+00	t	\\x6576a5dbaa09d20812adb277d875b55725871b5338f9681597cebb3968638b012a66b1a58345a4550fef64381928f502	18340838
\.


--
-- Data for Name: audit_logs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at) FROM stdin;
104f639f-7dfc-4fe8-be38-239d3b877bc1	00000000-0000-0000-0000-000000000001	system.install_completed	system	00000000-0000-0000-0000-000000000001	{"site_name": "Browser Regression", "admin_email": "admin@example.com", "mail_enabled": false, "storage_backend": "local", "favicon_configured": false}	\N	2026-04-07 13:07:52.952097+00
\.


--
-- Data for Name: auth_state; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.auth_state (id, session_epoch, updated_at) FROM stdin;
1	0	2026-04-07 13:07:49.623296+00
\.


--
-- Data for Name: email_verification_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.email_verification_tokens (id, user_id, token_hash, expires_at, used_at, created_at) FROM stdin;
\.


--
-- Data for Name: images; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.images (id, user_id, filename, thumbnail, size, hash, format, views, status, expires_at, created_at) FROM stdin;
\.


--
-- Data for Name: media_blobs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.media_blobs (created_at, storage_key, media_kind, content_hash, ref_count, status, updated_at) FROM stdin;
\.


--
-- Data for Name: password_reset_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.password_reset_tokens (id, user_id, token_hash, expires_at, used_at, created_at) FROM stdin;
\.


--
-- Data for Name: revoked_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.revoked_tokens (token_hash, expires_at, created_at) FROM stdin;
\.


--
-- Data for Name: settings; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.settings (key, value, updated_at) FROM stdin;
site_name	Browser Regression	2026-04-07 13:07:52.660773+00
storage_backend	local	2026-04-07 13:07:52.660773+00
local_storage_path	/data/images	2026-04-07 13:07:52.660773+00
mail_enabled	false	2026-04-07 13:07:52.660773+00
mail_smtp_host		2026-04-07 13:07:52.660773+00
mail_smtp_port	0	2026-04-07 13:07:52.660773+00
mail_from_email		2026-04-07 13:07:52.660773+00
mail_from_name		2026-04-07 13:07:52.660773+00
mail_link_base_url		2026-04-07 13:07:52.660773+00
system_installed	true	2026-04-07 13:07:52.660773+00
\.


--
-- Data for Name: storage_cleanup_jobs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.storage_cleanup_jobs (id, file_key, storage_signature, storage_snapshot, attempts, last_error, next_attempt_at, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.users (id, email, email_verified_at, password_hash, role, created_at, token_version) FROM stdin;
00000000-0000-0000-0000-000000000001	admin@example.com	2026-04-07 13:07:52.660773+00	$2b$12$PW3u7vYWSv1Of7bcTVu.n.xDj6MkGQTpjoagbSuzWmH5bx2HRAFiS	admin	2026-04-07 13:07:52.948973+00	0
\.


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: audit_logs audit_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_pkey PRIMARY KEY (id);


--
-- Name: auth_state auth_state_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.auth_state
    ADD CONSTRAINT auth_state_pkey PRIMARY KEY (id);


--
-- Name: email_verification_tokens email_verification_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_verification_tokens
    ADD CONSTRAINT email_verification_tokens_pkey PRIMARY KEY (id);


--
-- Name: email_verification_tokens email_verification_tokens_token_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_verification_tokens
    ADD CONSTRAINT email_verification_tokens_token_hash_key UNIQUE (token_hash);


--
-- Name: images images_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.images
    ADD CONSTRAINT images_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_pkey PRIMARY KEY (id);


--
-- Name: password_reset_tokens password_reset_tokens_token_hash_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_token_hash_key UNIQUE (token_hash);


--
-- Name: revoked_tokens revoked_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.revoked_tokens
    ADD CONSTRAINT revoked_tokens_pkey PRIMARY KEY (token_hash);


--
-- Name: settings settings_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.settings
    ADD CONSTRAINT settings_pkey PRIMARY KEY (key);


--
-- Name: storage_cleanup_jobs storage_cleanup_jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.storage_cleanup_jobs
    ADD CONSTRAINT storage_cleanup_jobs_pkey PRIMARY KEY (id);


--
-- Name: users users_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_email_key UNIQUE (email);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: idx_audit_logs_action; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_action ON public.audit_logs USING btree (action);


--
-- Name: idx_audit_logs_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_created_at ON public.audit_logs USING btree (created_at DESC);


--
-- Name: idx_audit_logs_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_audit_logs_user_id ON public.audit_logs USING btree (user_id);


--
-- Name: idx_email_verification_tokens_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_email_verification_tokens_expires_at ON public.email_verification_tokens USING btree (expires_at);


--
-- Name: idx_email_verification_tokens_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_email_verification_tokens_user_id ON public.email_verification_tokens USING btree (user_id);


--
-- Name: idx_images_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_created_at ON public.images USING btree (created_at DESC);


--
-- Name: idx_images_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_expires_at ON public.images USING btree (expires_at);


--
-- Name: idx_images_filename_lookup; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_filename_lookup ON public.images USING btree (filename);


--
-- Name: idx_images_filename_trgm; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_filename_trgm ON public.images USING gin (filename public.gin_trgm_ops);


--
-- Name: idx_images_hash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_hash ON public.images USING btree (hash);


--
-- Name: idx_images_size; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_size ON public.images USING btree (size);


--
-- Name: idx_images_status; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_status ON public.images USING btree (status);


--
-- Name: idx_images_status_expires; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_status_expires ON public.images USING btree (status, expires_at) WHERE (expires_at IS NOT NULL);


--
-- Name: idx_images_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_id ON public.images USING btree (user_id);


--
-- Name: idx_images_user_status_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_status_created ON public.images USING btree (user_id, status, created_at DESC);


--
-- Name: idx_images_views; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_views ON public.images USING btree (views DESC);


--
-- Name: idx_password_reset_tokens_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_password_reset_tokens_expires_at ON public.password_reset_tokens USING btree (expires_at);


--
-- Name: idx_password_reset_tokens_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_password_reset_tokens_user_id ON public.password_reset_tokens USING btree (user_id);


--
-- Name: idx_revoked_tokens_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_revoked_tokens_expires_at ON public.revoked_tokens USING btree (expires_at);


--
-- Name: idx_storage_cleanup_jobs_next_attempt_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_storage_cleanup_jobs_next_attempt_at ON public.storage_cleanup_jobs USING btree (next_attempt_at);


--
-- Name: uq_media_blobs_storage_key; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX uq_media_blobs_storage_key ON public.media_blobs USING btree (storage_key);


--
-- Name: uq_storage_cleanup_jobs_signature_file; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX uq_storage_cleanup_jobs_signature_file ON public.storage_cleanup_jobs USING btree (storage_signature, file_key);


--
-- Name: audit_logs audit_logs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: email_verification_tokens email_verification_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_verification_tokens
    ADD CONSTRAINT email_verification_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: password_reset_tokens password_reset_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict 7hRJ77ZVlQH3PV8UkbKzQUFfDZjO6WDFhuY3VRQC9HVCAs3MuTWslRwajoO8guN

