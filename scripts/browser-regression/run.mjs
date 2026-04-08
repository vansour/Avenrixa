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
  bootstrapDatabaseUrl:
    process.env.BROWSER_DATABASE_URL || process.env.POSTGRES_DATABASE_URL || "",
  backupFilename: process.env.BROWSER_BACKUP_FILENAME || "",
  storageStatePath: process.env.BROWSER_STORAGE_STATE_PATH || "",
  executablePath: process.env.BROWSER_EXECUTABLE_PATH || undefined,
  expectedDatabaseConnection: process.env.BROWSER_EXPECT_DATABASE_CONNECTION || "",
  artifactDir:
    process.env.BROWSER_REGRESSION_ARTIFACT_DIR ||
    path.join(process.cwd(), "tmp", "browser-regression-artifacts"),
};

const UPLOAD_FIXTURE = {
  name: "browser-regression-phase2.png",
  mimeType: "image/png",
  buffer: Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==",
    "base64"
  ),
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
          (item.endsWith(".sql") || item.endsWith(".mysql.sql"))
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

async function waitForImageCard(page, filename, timeout = PHASE_TIMEOUT_MS) {
  const card = page.locator("article.image-card", {
    has: page.getByText(filename, { exact: true }),
  }).first();
  await card.waitFor({ state: "visible", timeout });
  return card;
}

async function waitForUploadedFilename(page, timeout = PHASE_TIMEOUT_MS) {
  const title = page.locator(".upload-result-title").first();
  await title.waitFor({ state: "visible", timeout });
  const value = ((await title.textContent()) || "").trim();
  assert.ok(value, "uploaded filename should be visible in upload result card");
  return value;
}

async function waitForImageCardToDisappear(page, filename, timeout = PHASE_TIMEOUT_MS) {
  const card = page.locator("article.image-card", {
    has: page.getByText(filename, { exact: true }),
  }).first();
  await card.waitFor({ state: "hidden", timeout });
}

async function phaseBootstrapDatabase(page) {
  log("phase bootstrap-database");
  await gotoRoot(page);
  const bodyText = await waitForText(page, /数据库引导|安装向导/, "数据库引导页或安装向导");
  if (bodyText.includes("安装向导")) {
    log("database bootstrap skipped because compose already preconfigured DATABASE_URL");
    return { skipped: true };
  }

  assert.ok(
    config.bootstrapDatabaseUrl,
    "BROWSER_DATABASE_URL or POSTGRES_DATABASE_URL is required for bootstrap-database"
  );
  await fillInputByLabel(page, "数据库连接 URL", config.bootstrapDatabaseUrl);
  await clickButton(page, "保存数据库配置");
  await waitForText(
    page,
    "数据库配置已保存，请重启服务后继续安装",
    "PostgreSQL 数据库引导保存成功"
  );
  return {};
}

async function phaseInstallAndBackup(page) {
  log("phase install-and-backup");
  await gotoRoot(page);
  await waitForText(page, "安装向导", "安装向导");
  await waitForText(page, "数据库来源", "安装页数据库摘要");

  if (config.expectedDatabaseConnection) {
    await waitForText(page, config.expectedDatabaseConnection, "安装页数据库连接摘要");
  }

  await fillInputByLabel(page, "管理员邮箱", config.adminEmail);
  await fillInputByLabel(page, "管理员密码", config.adminPassword);
  await fillInputByLabel(page, "确认管理员密码", config.adminPassword);
  await fillInputByLabel(page, "站点名称", config.siteName);
  await fillInputByLabel(page, "本地存储路径", "/data/images");
  await clickButton(page, "完成安装");
  await waitForAuthenticatedNav(page);
  log("install phase: install completed and authenticated nav is visible");

  await openSettingsPage(page);
  await clickSettingsNav(page, "基础设置");
  assert.equal(
    await inputValueByLabel(page, "网站名称"),
    config.siteName,
    "settings should retain the installed site name"
  );
  assert.equal(
    await inputValueByLabel(page, "本地存储路径"),
    "/data/images",
    "settings should retain the installed storage path"
  );

  await clickSettingsNav(page, "维护");
  await waitForText(page, "创建数据库备份", "维护页面备份卡片");
  await clickButton(page, "创建备份");

  const backupFilename = await waitForBackupFilename(page);
  log(`captured backup filename: ${backupFilename}`);

  const backupRow = page.locator("article.settings-entity-card", {
    has: page.getByText(backupFilename, { exact: true }),
  });
  const backupRowText = (await backupRow.textContent()) || "";
  assert.match(
    backupRowText,
    /逻辑备份仅供下载；恢复统一走运维脚本/,
    "backup row should explain that restore is ops-only"
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

  await clickSettingsNav(page, "维护");
  await waitForText(page, config.backupFilename, "维护中的备份文件名");
  await waitForText(page, /逻辑导出/, "维护中的数据库类型文案");
  await waitForText(page, "逻辑备份仅供下载；恢复统一走运维脚本。", "维护中的恢复限制文案");

  await clickNavButtonRobust(page, "API 接入");
  await waitForText(page, "接入速查", "API 页面标题");
  await waitForText(page, "HttpOnly Cookie Session", "API 认证文案");
  await waitForText(page, "/api/v1/images?limit=20", "API 图片列表示例");

  return { backupFilename: config.backupFilename };
}

async function phaseAuthSemantics(page) {
  log("phase auth-semantics");

  await gotoRoot(page);
  await waitForAuthenticatedNav(page);
  log("auth phase: opened authenticated dashboard");
  await openSettingsPage(page);
  await clickSettingsNav(page, "安全");
  await waitForButton(page, "修改密码");
  log("auth phase: opened security settings");

  await fillInputByLabel(page, "当前密码", config.adminPassword);
  await fillInputByLabel(page, "新密码", config.adminNewPassword);
  await fillInputByLabel(page, "确认新密码", config.adminNewPassword);
  await clickButton(page, "修改密码");

  await waitForText(page, "登录控制台", "改密码后回到登录页");
  log("auth phase: password change forced login page");
  await waitForText(
    page,
    "当前站点未启用邮件能力，仅支持已有账号直接登录。",
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
  await clickSettingsNav(page, "用户管理");
  await waitForText(page, config.adminEmail, "用户管理中的管理员邮箱");
  await waitForText(page, "保存角色", "用户管理操作按钮");

  await clickSettingsNav(page, "系统状态");
  await waitForButton(page, "刷新状态");
  await waitForText(page, "运行操作指标", "系统状态运行指标区块");
  await waitForText(page, "运行积压", "系统状态运行积压区块");
  await waitForText(page, "后台任务", "系统状态后台任务区块");
  log("auth phase: opened system status before session expiry test");

  await page.context().clearCookies();
  await clickButtonRobust(page, "刷新状态");
  await waitForText(page, "登录控制台", "会话失效后返回登录页");
  log("auth phase: expired session redirected from system status");

  await login(page, config.adminNewPassword);
  await gotoRoot(page);
  await waitForAuthenticatedNav(page);
  log("auth phase: authenticated nav visible after relogin");

  const uploadInput = page.locator("#upload-file");
  await uploadInput.setInputFiles({
    name: UPLOAD_FIXTURE.name,
    mimeType: UPLOAD_FIXTURE.mimeType,
    buffer: UPLOAD_FIXTURE.buffer,
  });
  await clickButton(page, "开始上传");
  await waitForText(page, "上传成功:", "上传成功提示");
  await waitForText(page, "本次上传", "上传结果区块");
  const uploadedFilename = await waitForUploadedFilename(page);
  log("auth phase: uploaded image through upload page");

  await clickNavButtonRobust(page, "历史图库");
  log("auth phase: clicked history nav");
  await waitForText(page, "上传历史", "历史图库标题");
  await waitForButton(page, "刷新");
  await waitForImageCard(page, uploadedFilename);
  log("auth phase: uploaded image appeared in history page");

  page.once("dialog", (dialog) => dialog.accept());
  const uploadedCard = await waitForImageCard(page, uploadedFilename);
  await uploadedCard.getByRole("button", { name: "永久删除", exact: true }).click();
  await waitForImageCardToDisappear(page, uploadedFilename);
  log("auth phase: deleted uploaded image from history page");

  await clickButton(page, "刷新");
  await waitForText(page, "上传历史", "刷新后的历史图库标题");
  await waitForImageCardToDisappear(page, uploadedFilename);
  log("auth phase: history refresh kept deleted image absent");

  log("auth phase: opened history page");
  await page.context().clearCookies();
  await clickButton(page, "刷新");
  await waitForText(page, "登录控制台", "历史图库会话失效后返回登录页");
  log("auth phase: expired session redirected from history page");

  return {};
}

async function runPhase(page) {
  switch (phase) {
    case "bootstrap-postgres":
      return phaseBootstrapDatabase(page);
    case "bootstrap-mysql":
      return phaseBootstrapDatabase(page);
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
