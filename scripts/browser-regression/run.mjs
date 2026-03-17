import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const phase = process.argv[2];
const PHASE_TIMEOUT_MS = parseIntEnv("BROWSER_PHASE_TIMEOUT_MS", 45_000);
const HEADLESS = process.env.BROWSER_HEADLESS !== "0";

const config = {
  baseUrl: (process.env.BROWSER_BASE_URL || "http://127.0.0.1:8080").replace(/\/+$/, ""),
  adminEmail: requiredEnv("ADMIN_EMAIL"),
  adminPassword: requiredEnv("ADMIN_PASSWORD"),
  adminNewPassword: process.env.ADMIN_NEW_PASSWORD || "Password123456!updated",
  siteName: process.env.SITE_NAME || "Browser Regression",
  mysqlDatabaseUrl: requiredEnv("MYSQL_DATABASE_URL"),
  backupFilename: process.env.BROWSER_BACKUP_FILENAME || "",
  storageStatePath: process.env.BROWSER_STORAGE_STATE_PATH || "",
  executablePath: process.env.BROWSER_EXECUTABLE_PATH || undefined,
  expectedDatabaseConnection: process.env.BROWSER_EXPECT_DATABASE_CONNECTION || "",
  expectedCacheConnection: process.env.BROWSER_EXPECT_CACHE_CONNECTION || "",
  artifactDir:
    process.env.BROWSER_REGRESSION_ARTIFACT_DIR ||
    path.join(process.cwd(), "tmp", "browser-regression-artifacts"),
};

function parseIntEnv(name, fallback) {
  const value = process.env[name];
  const parsed = Number.parseInt(value ?? "", 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function requiredEnv(name) {
  const value = process.env[name];
  if (!value || !value.trim()) {
    throw new Error(`Missing required environment variable: ${name}`);
  }
  return value.trim();
}

function log(message) {
  console.error(`[browser-regression] ${message}`);
}

function textMatches(text, matcher) {
  if (matcher instanceof RegExp) {
    return matcher.test(text);
  }
  return text.includes(matcher);
}

async function waitForText(page, matcher, description = String(matcher), timeout = PHASE_TIMEOUT_MS) {
  const deadline = Date.now() + timeout;
  let lastBodyText = "";

  while (Date.now() < deadline) {
    const bodyText = await page.locator("body").textContent().catch(() => "");
    if (bodyText && textMatches(bodyText, matcher)) {
      return bodyText;
    }
    lastBodyText = bodyText || "";
    await page.waitForTimeout(250);
  }

  throw new Error(
    `Timed out waiting for text: ${description}; body=${lastBodyText
      .replace(/\s+/g, " ")
      .slice(0, 400)}`
  );
}

async function waitForAuthenticatedNav(page, timeout = PHASE_TIMEOUT_MS) {
  const authNavLabels = ["上传中心", "历史图库", "API 接入", "系统设置"];
  const deadline = Date.now() + timeout;
  let lastBodyText = "";

  while (Date.now() < deadline) {
    for (const label of authNavLabels) {
      const button = page.getByRole("button", { name: label, exact: true }).first();
      if (await button.isVisible().catch(() => false)) {
        return;
      }
    }

    lastBodyText = (await page.locator("body").textContent().catch(() => "")) || "";
    await page.waitForTimeout(250);
  }

  throw new Error(`登录后未进入已认证导航: ${lastBodyText.slice(0, 400)}`);
}

async function waitForHeading(page, matcher, timeout = PHASE_TIMEOUT_MS) {
  const deadline = Date.now() + timeout;

  while (Date.now() < deadline) {
    const texts = await page.locator("h1, h2, h3").allTextContents().catch(() => []);
    if (texts.some((item) => textMatches(item.trim(), matcher))) {
      return;
    }
    await page.waitForTimeout(250);
  }

  throw new Error(`Timed out waiting for heading: ${matcher}`);
}

async function waitForStorageBrowserPath(page, matcher = null, timeout = PHASE_TIMEOUT_MS) {
  const currentPath = page.locator(".install-path-browser-current").first();
  const deadline = Date.now() + timeout;
  let lastPath = "";

  while (Date.now() < deadline) {
    const text = ((await currentPath.textContent().catch(() => "")) || "").trim();
    if (text) {
      lastPath = text;
      if (!matcher || textMatches(text, matcher)) {
        return text;
      }
    }
    await page.waitForTimeout(250);
  }

  throw new Error(`Timed out waiting for storage browser path: ${matcher ?? "any"}; last=${lastPath}`);
}

async function clickStorageBrowserDirectory(page, name) {
  const directoryButton = page.locator("button.install-path-browser-item", {
    has: page.locator(".install-path-browser-name", { hasText: name }),
  });
  await directoryButton.first().click();
}

async function selectInstallStorageDirectory(page) {
  let currentPath = await waitForStorageBrowserPath(page);
  if (currentPath === "/") {
    await clickStorageBrowserDirectory(page, "data");
    currentPath = await waitForStorageBrowserPath(page, "/data");
  }

  if (currentPath === "/data") {
    await clickStorageBrowserDirectory(page, "images");
    currentPath = await waitForStorageBrowserPath(page, "/data/images");
  }

  if (currentPath === "/data/images") {
    return currentPath;
  }

  throw new Error(`Unexpected install storage browser path: ${currentPath}`);
}

async function gotoRoot(page) {
  await page.goto(`${config.baseUrl}/`, { waitUntil: "domcontentloaded" });
}

async function clickButton(page, name) {
  await page.getByRole("button", { name, exact: true }).click();
}

async function clickAnyButton(page, names) {
  for (const name of names) {
    const button = page.getByRole("button", { name, exact: true }).first();
    if (await button.isVisible().catch(() => false)) {
      await button.click();
      return name;
    }
  }

  throw new Error(`None of the expected buttons were visible: ${names.join(", ")}`);
}

async function waitForButton(page, name, timeout = PHASE_TIMEOUT_MS) {
  const button = page.getByRole("button", { name, exact: true }).first();
  await button.waitFor({ state: "visible", timeout });
}

async function clickButtonRobust(page, name) {
  const button = page.getByRole("button", { name, exact: true }).first();
  const isVisible = await button.isVisible().catch(() => false);
  if (isVisible) {
    try {
      await button.click();
      return;
    } catch (error) {
      log(`button click fallback for ${name}: ${error}`);
    }
  }

  await clickButtonViaDom(page, name);
}

async function tryClickButtonViaDom(page, name) {
  return page.evaluate((name) => {
    const dispatchClick = (target) => {
      for (const type of ["pointerdown", "mousedown", "pointerup", "mouseup", "click"]) {
        target.dispatchEvent(
          new MouseEvent(type, {
            bubbles: true,
            cancelable: true,
            view: window,
          })
        );
      }
    };

    const button = Array.from(document.querySelectorAll("button")).find((candidate) => {
      const text = candidate.textContent?.trim();
      return text === name;
    });
    if (!button) {
      return false;
    }
    dispatchClick(button);
    return true;
  }, name);
}

async function clickButtonViaDom(page, name) {
  const clicked = await tryClickButtonViaDom(page, name);
  assert.equal(clicked, true, `button ${name} should exist`);
}

async function clickNavButton(page, name) {
  const clicked = await page.evaluate((name) => {
    const dispatchClick = (target) => {
      for (const type of ["pointerdown", "mousedown", "pointerup", "mouseup", "click"]) {
        target.dispatchEvent(
          new MouseEvent(type, {
            bubbles: true,
            cancelable: true,
            view: window,
          })
        );
      }
    };

    const button = Array.from(document.querySelectorAll("nav button")).find((candidate) => {
      const text = candidate.textContent?.trim();
      return text === name;
    });
    if (!button) {
      return false;
    }
    window.setTimeout(() => dispatchClick(button), 0);
    return true;
  }, name);

  assert.equal(clicked, true, `navigation button ${name} should exist`);
  await page.waitForTimeout(50);
}

async function ensureNavButtonVisible(page, name) {
  const button = page.getByRole("button", { name, exact: true }).first();
  if (await button.isVisible().catch(() => false)) {
    return button;
  }

  const toggle = page.locator("button.navbar-toggle").first();
  if (await toggle.isVisible().catch(() => false)) {
    await toggle.click();
    await button.waitFor({ state: "visible", timeout: 3_000 }).catch(() => {});
  }

  return button;
}

async function clickNavButtonRobust(page, name) {
  await ensureNavButtonVisible(page, name);
  await clickNavButton(page, name);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function clickLoginSubmit(page) {
  await page.locator("main").getByRole("button", { name: "登录", exact: true }).click();
}

async function requestApi(page, path, options = {}) {
  return page.evaluate(async ({ path, options }) => {
    const headers = { Accept: "application/json", ...(options.headers || {}) };
    if (options.body && !headers["Content-Type"]) {
      headers["Content-Type"] = "application/json";
    }

    const response = await fetch(path, {
      method: options.method || "GET",
      credentials: "include",
      headers,
      body: options.body ?? undefined,
    });
    const text = await response.text();
    return { status: response.status, text };
  }, { path, options });
}

async function login(page, password) {
  await gotoRoot(page);
  await waitForText(page, "登录控制台", "登录页");
  await fillInputByLabel(page, "邮箱", config.adminEmail);
  await fillInputByLabel(page, "密码", password);
  await clickLoginSubmit(page);
  await waitForAuthenticatedNav(page);
}

async function findControlByLabel(page, label, selector) {
  const labelNode = page.locator("label", { hasText: label }).first();
  await labelNode.waitFor({ state: "visible", timeout: PHASE_TIMEOUT_MS });

  const nestedControl = labelNode.locator(selector).first();
  if ((await nestedControl.count()) > 0) {
    await nestedControl.waitFor({ state: "visible", timeout: PHASE_TIMEOUT_MS });
    return nestedControl;
  }

  const fieldId = await labelNode.getAttribute("for");
  if (fieldId) {
    const control = page.locator(`[id="${fieldId}"]`).first();
    await control.waitFor({ state: "visible", timeout: PHASE_TIMEOUT_MS });
    return control;
  }

  throw new Error(`Could not resolve control for label: ${label}`);
}

async function fillInputByLabel(page, label, value) {
  const input = await findControlByLabel(page, label, "input, textarea");
  await input.fill(value);
}

async function inputValueByLabel(page, label) {
  const input = await findControlByLabel(page, label, "input, textarea");
  return input.inputValue();
}

async function inputValueByAnyLabel(page, labels) {
  for (const label of labels) {
    const labelNode = page.locator("label", { hasText: label }).first();
    const exists = (await labelNode.count().catch(() => 0)) > 0;
    if (!exists) {
      continue;
    }
    const isVisible = await labelNode.isVisible().catch(() => false);
    if (isVisible) {
      return inputValueByLabel(page, label);
    }
  }

  throw new Error(`Could not resolve input for any label: ${labels.join(", ")}`);
}

async function selectOptionByLabel(page, label, value) {
  const select = await findControlByLabel(page, label, "select");
  await select.selectOption(value);
}

async function clickSettingsNav(page, label) {
  const button = page.locator(`nav.settings-nav button:has-text("${label}")`).first();
  await button.waitFor({ state: "visible", timeout: PHASE_TIMEOUT_MS });
  await button.click();
}

async function reopenFirstRunGuide(page) {
  await page.evaluate((email) => {
    window.localStorage.removeItem(
      `avenrixa:first-run-guide:v1:${email.trim().toLowerCase()}`
    );
  }, config.adminEmail);
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForText(page, "首次进入引导", "首次进入引导弹窗");
}

async function assertGuideTarget(page, buttonName, matcher) {
  await clickButton(page, buttonName);
  await waitForText(page, matcher, `首次引导目标 ${buttonName}`);
}

async function waitForBackupFilename(page) {
  const deadline = Date.now() + PHASE_TIMEOUT_MS;

  while (Date.now() < deadline) {
    const headings = await page
      .locator("article.settings-entity-card h3")
      .allTextContents()
      .catch(() => []);
    const match = headings
      .map((item) => item.trim())
      .find(
        (item) =>
          /^backup_/i.test(item) &&
          !item.startsWith("rollback_before_restore_") &&
          (item.endsWith(".sql") || item.endsWith(".mysql.sql") || item.endsWith(".sqlite3"))
      );
    if (match) {
      return match;
    }
    await page.waitForTimeout(500);
  }

  throw new Error("Timed out waiting for backup filename in maintenance list");
}

async function openSettingsPage(page) {
  await clickButton(page, "系统设置");
  await waitForHeading(page, "系统设置");
}

async function phaseBootstrapMysql(page) {
  log("phase bootstrap-mysql");
  await gotoRoot(page);
  const bodyText = await waitForText(page, /数据库引导|安装向导/, "数据库引导页或安装向导");
  if (bodyText.includes("安装向导")) {
    log("database bootstrap skipped because compose already preconfigured DATABASE_URL");
    return { skipped: true };
  }

  await selectOptionByLabel(page, "数据库类型", "mysql");
  await fillInputByLabel(page, "数据库连接 URL", config.mysqlDatabaseUrl);
  await clickButton(page, "保存 MySQL 配置");
  await waitForText(
    page,
    "MySQL / MariaDB 配置已保存，请重启服务后继续安装",
    "MySQL 数据库引导保存成功"
  );
  return {};
}

async function phaseInstallAndBackup(page) {
  log("phase install-and-backup");
  await gotoRoot(page);
  await waitForText(page, "安装向导", "安装向导");
  await waitForText(page, /当前(?:步骤|进行中)\s*1\/4/, "安装步骤进度");
  await waitForText(page, "部署环境", "部署环境步骤");
  await waitForText(page, "创建管理员账号", "管理员步骤");

  const environmentItems = page.locator(".install-env-inline-item");
  assert.equal(await environmentItems.count(), 2, "wizard should show database and cache summaries");

  const databaseSummary = page
    .locator(".install-env-inline-item", {
      has: page.locator(".install-env-inline-label", { hasText: "数据库" }),
    })
    .first();
  const cacheSummary = page
    .locator(".install-env-inline-item", {
      has: page.locator(".install-env-inline-label", { hasText: "缓存" }),
    })
    .first();
  const databaseSummaryText = ((await databaseSummary.textContent()) || "").replace(/\s+/g, " ");
  const cacheSummaryText = ((await cacheSummary.textContent()) || "").replace(/\s+/g, " ");

  assert.match(
    databaseSummaryText,
    new RegExp(escapeRegExp(config.expectedDatabaseConnection)),
    "database summary should match the masked runtime connection"
  );
  assert.match(
    cacheSummaryText,
    new RegExp(escapeRegExp(config.expectedCacheConnection)),
    "cache summary should match the masked runtime connection"
  );

  await fillInputByLabel(page, "管理员邮箱", config.adminEmail);
  await fillInputByLabel(page, "管理员密码", config.adminPassword);
  await fillInputByLabel(page, "确认密码", config.adminPassword);
  await clickAnyButton(page, ["下一步：配置站点信息", "下一步：站点信息"]);

  await waitForText(page, /当前(?:步骤|进行中)\s*2\/4/, "站点信息步骤进度");
  await fillInputByLabel(page, "网站名称", config.siteName);
  await page
    .locator('label.settings-check', { hasText: "启用邮件服务" })
    .locator('input[type="checkbox"]')
    .check();
  assert.match(
    await inputValueByAnyLabel(page, ["站点访问地址（用于邮件链接）", "站点访问地址（必填）"]),
    new RegExp(`^(?:|${escapeRegExp(config.baseUrl)})$`),
    "mail link base url should be blank or default to current origin"
  );
  await page
    .locator('label.settings-check', { hasText: "启用邮件服务" })
    .locator('input[type="checkbox"]')
    .uncheck();
  await clickAnyButton(page, ["下一步：确认存储方案", "下一步：存储后端"]);

  await waitForText(page, /当前(?:步骤|进行中)\s*3\/4/, "存储后端步骤进度");
  await selectOptionByLabel(page, "存储后端", "local");
  await waitForText(page, "本地存储路径", "本地存储路径字段");
  await clickAnyButton(page, ["浏览", "选择文件夹"]);
  await waitForText(page, "选择本地存储目录", "本地目录选择器");
  await selectInstallStorageDirectory(page);
  await clickButton(page, "选择当前文件夹");
  await clickAnyButton(page, ["下一步：检查并初始化", "下一步：最终确认"]);

  await waitForText(page, /当前(?:步骤|进行中)\s*4\/4/, "最终确认步骤进度");
  await clickAnyButton(page, ["完成安装并创建管理员", "完成安装"]);
  await waitForAuthenticatedNav(page);
  log("install phase: install completed and authenticated nav is visible");

  await openSettingsPage(page);
  await clickSettingsNav(page, "维护工具");
  await waitForText(page, "数据库恢复状态", "维护工具恢复状态卡片");
  await clickButton(page, "生成备份");

  const backupFilename = await waitForBackupFilename(page);
  log(`captured backup filename: ${backupFilename}`);

  const backupRow = page.locator("article.settings-entity-card", {
    has: page.getByText(backupFilename, { exact: true }),
  });
  const backupRowText = (await backupRow.textContent()) || "";
  assert.match(
    backupRowText,
    /仅支持下载或运维恢复/,
    "backup row should explain that page restore is unavailable"
  );

  const restoreButton = backupRow
    .getByRole("button", { name: /不支持页面恢复|暂不支持恢复/ })
    .first();
  assert.equal(
    await restoreButton.isDisabled(),
    true,
    "backup row should expose a disabled restore button"
  );

  return { backupFilename };
}

async function phaseVerifyBackupAudit(page) {
  log("phase verify-backup-audit");
  assert.ok(config.backupFilename, "BROWSER_BACKUP_FILENAME is required for verify-backup-audit");

  await gotoRoot(page);
  await waitForAuthenticatedNav(page);
  await openSettingsPage(page);

  const auditResponse = await requestApi(page, "/api/v1/audit-logs?page=1&page_size=100");
  assert.equal(auditResponse.status, 200, "admin session should load audit logs");
  const auditPayload = JSON.parse(auditResponse.text);
  const backupAudit = (auditPayload.data || []).find(
    (entry) =>
      entry.action === "admin.maintenance.database_backup.created" &&
      entry.details &&
      entry.details.filename === config.backupFilename
  );
  assert.ok(backupAudit, "audit logs should contain the created backup entry");
  log("verify-backup-audit phase: backup audit entry verified via API");

  await clickSettingsNav(page, "维护工具");
  await waitForText(page, config.backupFilename, "维护工具中的备份文件名");
  await waitForText(page, /逻辑导出|SQLite 数据库快照/, "维护工具数据库类型文案");
  await waitForText(page, "仅支持下载或运维恢复", "维护工具恢复限制文案");

  return { backupFilename: config.backupFilename };
}

async function phaseAuthSemantics(page) {
  log("phase auth-semantics");

  await gotoRoot(page);
  await waitForAuthenticatedNav(page);
  log("auth phase: opened authenticated dashboard");
  await openSettingsPage(page);
  await clickSettingsNav(page, "账号安全");
  await waitForText(page, "修改密码", "账号安全分区");
  log("auth phase: opened security settings");

  await fillInputByLabel(page, "当前密码", config.adminPassword);
  await fillInputByLabel(page, "新密码", config.adminNewPassword);
  await fillInputByLabel(page, "确认新密码", config.adminNewPassword);
  await clickButton(page, "修改密码");

  await waitForText(page, "登录控制台", "改密码后回到登录页");
  log("auth phase: password change forced login page");
  await waitForText(
    page,
    "注册和密码找回入口会在邮件能力启用后开放。",
    "未启用邮件时的登录页能力提示"
  );
  assert.equal(
    await page.getByRole("button", { name: "注册新账号", exact: true }).count(),
    0,
    "mail disabled login page should not show register entry"
  );
  assert.equal(
    await page.getByRole("button", { name: "忘记密码", exact: true }).count(),
    0,
    "mail disabled login page should not show password reset entry"
  );

  await fillInputByLabel(page, "邮箱", config.adminEmail);
  await fillInputByLabel(page, "密码", config.adminPassword);
  await clickLoginSubmit(page);
  await waitForText(page, "登录失败", "旧密码登录失败提示");
  log("auth phase: old password rejected");

  await fillInputByLabel(page, "密码", config.adminNewPassword);
  await clickLoginSubmit(page);
  await waitForAuthenticatedNav(page);
  log("auth phase: relogin with new password succeeded");

  await openSettingsPage(page);
  await clickSettingsNav(page, "系统状态");
  await waitForButton(page, "刷新状态");
  log("auth phase: opened system status before session expiry test");

  await page.context().clearCookies();
  await clickButtonRobust(page, "刷新状态");
  await waitForText(page, "登录控制台", "会话失效后返回登录页");
  log("auth phase: expired session redirected from system status");

  await login(page, config.adminNewPassword);
  await openSettingsPage(page);
  await clickSettingsNav(page, "系统状态");
  await waitForButton(page, "刷新状态");
  log("auth phase: reopened system status before self-demotion");

  const usersResponse = await requestApi(page, "/api/v1/users");
  assert.equal(usersResponse.status, 200, "admin session should load users before demotion");
  const users = JSON.parse(usersResponse.text);
  const currentAdmin = users.find((user) => user.email === config.adminEmail);
  assert.ok(currentAdmin, "current admin should be present in users list");
  assert.ok(
    users.some(
      (user) =>
        user.email !== config.adminEmail &&
        String(user.role || "").toLowerCase() === "admin"
    ),
    "demotion regression requires another admin account"
  );

  const demoteResponse = await requestApi(
    page,
    `/api/v1/users/${encodeURIComponent(currentAdmin.id)}`,
    {
      method: "PUT",
      body: JSON.stringify({ role: "user" }),
    }
  );
  assert.equal(demoteResponse.status, 200, "self-demotion should succeed when another admin exists");
  log("auth phase: self-demotion request succeeded");

  await clickButtonRobust(page, "刷新状态");
  await waitForText(page, "登录控制台", "降权后访问后台返回登录页");
  log("auth phase: demoted admin redirected from settings");

  await login(page, config.adminNewPassword);
  log("auth phase: relogin as demoted user succeeded");
  await gotoRoot(page);
  await waitForAuthenticatedNav(page);
  log("auth phase: authenticated nav visible for demoted user");
  await clickNavButtonRobust(page, "历史图库");
  log("auth phase: clicked history nav as non-admin");
  await waitForText(page, "上传历史", "非管理员历史图库");
  log("auth phase: opened history page as non-admin");
  await page.context().clearCookies();
  await page.reload({ waitUntil: "domcontentloaded" });
  await waitForText(page, "登录控制台", "非设置后台页会话失效后返回登录页");
  log("auth phase: expired session redirected from history page");

  return {};
}

async function runPhase(page) {
  switch (phase) {
    case "bootstrap-mysql":
      return phaseBootstrapMysql(page);
    case "install-and-backup":
      return phaseInstallAndBackup(page);
    case "verify-backup-audit":
      return phaseVerifyBackupAudit(page);
    case "auth-semantics":
      return phaseAuthSemantics(page);
    default:
      throw new Error(`Unsupported browser regression phase: ${phase}`);
  }
}

async function writeFailureArtifact(page, error) {
  fs.mkdirSync(config.artifactDir, { recursive: true });
  const screenshotPath = path.join(
    config.artifactDir,
    `${phase || "unknown-phase"}-failure.png`
  );
  const failureUrl = page.url();
  const failureBodyText = ((await page.locator("body").textContent().catch(() => "")) || "")
    .replace(/\s+/g, " ")
    .slice(0, 800);
  const failureHtml = ((await page.content().catch(() => "")) || "")
    .replace(/\s+/g, " ")
    .slice(0, 800);
  await page.screenshot({ path: screenshotPath, fullPage: true }).catch(() => {});
  console.error(error.stack || String(error));
  console.error(`[browser-regression] failure url: ${failureUrl}`);
  console.error(`[browser-regression] failure body: ${failureBodyText}`);
  console.error(`[browser-regression] failure html: ${failureHtml}`);
  console.error(`[browser-regression] failure screenshot: ${screenshotPath}`);
}

async function main() {
  let browser;
  let context;
  let page;

  try {
    browser = await chromium.launch({
      headless: HEADLESS,
      executablePath: config.executablePath,
    });

    const contextOptions = {
      baseURL: config.baseUrl,
      ignoreHTTPSErrors: true,
      viewport: { width: 1440, height: 1100 },
    };
    if (config.storageStatePath && fs.existsSync(config.storageStatePath)) {
      contextOptions.storageState = config.storageStatePath;
    }

    context = await browser.newContext(contextOptions);
    page = await context.newPage();
    page.setDefaultTimeout(PHASE_TIMEOUT_MS);
    page.on("pageerror", (error) => {
      log(`pageerror: ${error.stack || String(error)}`);
    });
    page.on("console", (message) => {
      if (message.type() === "error") {
        log(`console.${message.type()}: ${message.text()}`);
      }
    });

    const result = await runPhase(page);
    if (config.storageStatePath) {
      fs.mkdirSync(path.dirname(config.storageStatePath), { recursive: true });
      await context.storageState({ path: config.storageStatePath });
    }
    console.log(JSON.stringify(result ?? {}));
  } catch (error) {
    if (page) {
      await writeFailureArtifact(page, error);
    } else {
      console.error(error.stack || String(error));
    }
    process.exitCode = 1;
  } finally {
    await context?.close();
    await browser?.close();
  }
}

await main();
