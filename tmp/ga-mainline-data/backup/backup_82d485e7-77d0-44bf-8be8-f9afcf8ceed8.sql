--
-- PostgreSQL database dump
--

\restrict uWn87pzwX2e7xsNgaePWhMyZRnKfobff3QlYpufMPRqp4GfunF5SaQF87YLBaf0

-- Dumped from database version 17.9 (Debian 17.9-1.pgdg13+1)
-- Dumped by pg_dump version 17.8 (Debian 17.8-0+deb13u1)

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
-- Name: categories; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.categories (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    name character varying(100) NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
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
-- Name: image_tags; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.image_tags (
    image_id uuid,
    tag character varying(50) NOT NULL
);


--
-- Name: images; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.images (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    category_id uuid,
    filename character varying(255) NOT NULL,
    thumbnail character varying(255),
    original_filename character varying(255),
    size bigint NOT NULL,
    hash character varying(64) NOT NULL,
    format character varying(20),
    views integer DEFAULT 0,
    status character varying(20) DEFAULT 'active'::character varying,
    expires_at timestamp with time zone,
    deleted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
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
1	initial schema	2026-03-12 10:26:23.747869+00	t	\\xe3d46ad709293dccd10547fadd6cde8ce8d4874417609b40088540b5c0e817ae5f1ef737d7853defcd7298ff4a2e7e93	32198031
2	drop unique filename index	2026-03-12 10:26:23.781189+00	t	\\x9c089c99b82b83f67e4a9fe2ff47b4483c1475ba7be56e320a2910efd939504c9a57b87f390151db31da31fa0ea79dc8	2844958
3	add auth runtime state	2026-03-12 10:26:23.784719+00	t	\\xf331d638badd2003158f875db2ec16a71a6b0e8721e5b605be725ee7c0f327eeaa32e65ccd2c2be3e4caa53c4cac516a	3251923
\.


--
-- Data for Name: audit_logs; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.audit_logs (id, user_id, action, target_type, target_id, details, ip_address, created_at) FROM stdin;
56ec67c0-321f-44ea-8ae9-23c17e353dfc	00000000-0000-0000-0000-000000000001	system.install_completed	system	00000000-0000-0000-0000-000000000001	{"site_name": "GA Acceptance", "admin_email": "ga@example.com", "mail_enabled": false, "storage_backend": "local", "favicon_configured": false}	\N	2026-03-12 10:26:44.494987+00
\.


--
-- Data for Name: auth_state; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.auth_state (id, session_epoch, updated_at) FROM stdin;
1	0	2026-03-12 10:26:23.784719+00
\.


--
-- Data for Name: categories; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.categories (id, user_id, name, created_at) FROM stdin;
\.


--
-- Data for Name: email_verification_tokens; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.email_verification_tokens (id, user_id, token_hash, expires_at, used_at, created_at) FROM stdin;
\.


--
-- Data for Name: image_tags; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.image_tags (image_id, tag) FROM stdin;
\.


--
-- Data for Name: images; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.images (id, user_id, category_id, filename, thumbnail, original_filename, size, hash, format, views, status, expires_at, deleted_at, created_at) FROM stdin;
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
site_name	GA Acceptance	2026-03-12 10:26:44.189879+00
storage_backend	local	2026-03-12 10:26:44.189879+00
local_storage_path	/data/images	2026-03-12 10:26:44.189879+00
mail_enabled	false	2026-03-12 10:26:44.189879+00
mail_smtp_host		2026-03-12 10:26:44.189879+00
mail_smtp_port	0	2026-03-12 10:26:44.189879+00
mail_from_email		2026-03-12 10:26:44.189879+00
mail_from_name		2026-03-12 10:26:44.189879+00
mail_link_base_url	http://127.0.0.1:18081	2026-03-12 10:26:44.189879+00
s3_force_path_style	false	2026-03-12 10:26:44.189879+00
system_installed	true	2026-03-12 10:26:44.189879+00
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: -
--

COPY public.users (id, email, email_verified_at, password_hash, role, created_at, token_version) FROM stdin;
00000000-0000-0000-0000-000000000001	ga@example.com	2026-03-12 10:26:44.189879+00	$2b$12$8i8co7DWm/NH2wpVqYFvwOA0zgFwCTEFTXdfmrLqRt9RCQlQIZ7Rm	admin	2026-03-12 10:26:44.488838+00	0
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
-- Name: categories categories_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_pkey PRIMARY KEY (id);


--
-- Name: categories categories_user_id_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_user_id_name_key UNIQUE (user_id, name);


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
-- Name: idx_categories_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_categories_created_at ON public.categories USING btree (created_at DESC);


--
-- Name: idx_categories_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_categories_user_id ON public.categories USING btree (user_id);


--
-- Name: idx_email_verification_tokens_expires_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_email_verification_tokens_expires_at ON public.email_verification_tokens USING btree (expires_at);


--
-- Name: idx_email_verification_tokens_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_email_verification_tokens_user_id ON public.email_verification_tokens USING btree (user_id);


--
-- Name: idx_image_tags_image_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_image_tags_image_id ON public.image_tags USING btree (image_id);


--
-- Name: idx_image_tags_image_tag; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_image_tags_image_tag ON public.image_tags USING btree (image_id, tag);


--
-- Name: idx_image_tags_tag; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_image_tags_tag ON public.image_tags USING btree (tag);


--
-- Name: idx_image_tags_tag_trgm; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_image_tags_tag_trgm ON public.image_tags USING gin (tag public.gin_trgm_ops);


--
-- Name: idx_images_category_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_category_id ON public.images USING btree (category_id);


--
-- Name: idx_images_created_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_created_at ON public.images USING btree (created_at DESC);


--
-- Name: idx_images_deleted_at; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_deleted_at ON public.images USING btree (deleted_at);


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
-- Name: idx_images_hash_user_deleted; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_hash_user_deleted ON public.images USING btree (hash, user_id, deleted_at) WHERE (deleted_at IS NULL);


--
-- Name: idx_images_original_filename_trgm; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_original_filename_trgm ON public.images USING gin (original_filename public.gin_trgm_ops) WHERE (original_filename IS NOT NULL);


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
-- Name: idx_images_user_category_deleted; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_category_deleted ON public.images USING btree (user_id, category_id, deleted_at);


--
-- Name: idx_images_user_category_status_partial; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_category_status_partial ON public.images USING btree (user_id, category_id, status) WHERE (deleted_at IS NULL);


--
-- Name: idx_images_user_deleted_at_partial; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_deleted_at_partial ON public.images USING btree (user_id, deleted_at DESC) WHERE (deleted_at IS NOT NULL);


--
-- Name: idx_images_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_id ON public.images USING btree (user_id);


--
-- Name: idx_images_user_status_created; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_status_created ON public.images USING btree (user_id, status, created_at DESC);


--
-- Name: idx_images_user_status_created_partial; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_status_created_partial ON public.images USING btree (user_id, status, created_at DESC) WHERE (deleted_at IS NULL);


--
-- Name: idx_images_user_status_expires_partial; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_status_expires_partial ON public.images USING btree (user_id, status, expires_at) WHERE ((deleted_at IS NULL) AND ((status)::text = 'active'::text));


--
-- Name: idx_images_user_status_partial; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_images_user_status_partial ON public.images USING btree (user_id, status) WHERE (deleted_at IS NULL);


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
-- Name: audit_logs audit_logs_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.audit_logs
    ADD CONSTRAINT audit_logs_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: categories categories_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.categories
    ADD CONSTRAINT categories_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: email_verification_tokens email_verification_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.email_verification_tokens
    ADD CONSTRAINT email_verification_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: image_tags image_tags_image_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.image_tags
    ADD CONSTRAINT image_tags_image_id_fkey FOREIGN KEY (image_id) REFERENCES public.images(id) ON DELETE CASCADE;


--
-- Name: password_reset_tokens password_reset_tokens_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict uWn87pzwX2e7xsNgaePWhMyZRnKfobff3QlYpufMPRqp4GfunF5SaQF87YLBaf0

