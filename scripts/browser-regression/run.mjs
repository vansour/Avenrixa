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
  siteName: process.env.SITE_NAME || "Browser Regression",
  mysqlDatabaseUrl: requiredEnv("MYSQL_DATABASE_URL"),
  backupFilename: process.env.BROWSER_BACKUP_FILENAME || "",
  storageStatePath: process.env.BROWSER_STORAGE_STATE_PATH || "",
  executablePath: process.env.BROWSER_EXECUTABLE_PATH || undefined,
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

  while (Date.now() < deadline) {
    const bodyText = await page.locator("body").textContent().catch(() => "");
    if (bodyText && textMatches(bodyText, matcher)) {
      return bodyText;
    }
    await page.waitForTimeout(250);
  }

  throw new Error(`Timed out waiting for text: ${description}`);
}

async function waitForAuthenticatedNav(page, timeout = PHASE_TIMEOUT_MS) {
  const settingsButton = page.getByRole("button", { name: "系统设置", exact: true });
  const deadline = Date.now() + timeout;
  let lastBodyText = "";

  while (Date.now() < deadline) {
    if (await settingsButton.isVisible().catch(() => false)) {
      return;
    }

    lastBodyText = (await page.locator("body").textContent().catch(() => "")) || "";
    if (lastBodyText.includes("登录失败")) {
      throw new Error(`登录后仍停留在未认证页面: ${lastBodyText.slice(0, 400)}`);
    }

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

async function gotoRoot(page) {
  await page.goto(`${config.baseUrl}/`, { waitUntil: "domcontentloaded" });
}

async function clickButton(page, name) {
  await page.getByRole("button", { name, exact: true }).click();
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
      `vansour-image:first-run-guide:v1:${email.trim().toLowerCase()}`
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
      .find((item) => /\.mysql\.sql$/i.test(item) && !item.startsWith("rollback_before_restore_"));
    if (match) {
      return match;
    }
    await page.waitForTimeout(500);
  }

  throw new Error("Timed out waiting for MySQL backup filename in maintenance list");
}

async function openSettingsPage(page) {
  await clickButton(page, "系统设置");
  await waitForHeading(page, "系统设置");
}

async function phaseBootstrapMysql(page) {
  log("phase bootstrap-mysql");
  await gotoRoot(page);
  await waitForText(page, "数据库引导", "数据库引导页");
  await selectOptionByLabel(page, "数据库类型", "mysql");
  await fillInputByLabel(page, "最大连接数（可选）", "5");
  await fillInputByLabel(page, "数据库连接 URL", config.mysqlDatabaseUrl);
  await clickButton(page, "保存 MySQL 配置");
  await waitForText(
    page,
    "MySQL / MariaDB 配置已保存，请重启服务后继续安装",
    "MySQL 数据库引导保存成功"
  );
  return {};
}

async function phaseInstallAndRestorePlan(page) {
  log("phase install-and-restore-plan");
  await gotoRoot(page);
  await waitForText(page, "安装向导", "安装向导");
  await fillInputByLabel(page, "管理员邮箱", config.adminEmail);
  await fillInputByLabel(page, "管理员密码", config.adminPassword);
  await fillInputByLabel(page, "确认密码", config.adminPassword);
  await fillInputByLabel(page, "网站名称", config.siteName);
  await clickButton(page, "完成安装");

  await waitForText(page, "首次进入引导", "首次进入引导弹窗");

  await assertGuideTarget(page, "打开基础设置", "基础设置");
  await reopenFirstRunGuide(page);

  await assertGuideTarget(page, "打开存储设置", "存储设置");
  await reopenFirstRunGuide(page);

  await assertGuideTarget(page, "去上传中心", "点击或拖拽上传图片");
  await reopenFirstRunGuide(page);

  await assertGuideTarget(page, "打开审计日志", "审计日志");

  await clickSettingsNav(page, "维护工具");
  await waitForText(page, "数据库恢复状态", "维护工具恢复状态卡片");
  await clickButton(page, "生成备份");

  const backupFilename = await waitForBackupFilename(page);
  log(`captured backup filename: ${backupFilename}`);

  const backupRow = page.locator("article.settings-entity-card", {
    has: page.getByText(backupFilename, { exact: true }),
  });
  await backupRow.getByRole("button", { name: "恢复到此备份", exact: true }).click();

  await waitForText(page, "写入 MySQL / MariaDB 恢复计划", "恢复确认弹窗标题");
  await waitForText(page, "文件回滚锚点：", "本地文件回滚锚点文案");

  const confirmInput = page.locator(".confirm-field input").first();
  await confirmInput.fill(backupFilename);
  await clickButton(page, "写入恢复计划");
  await waitForText(page, "检测到待执行的数据库恢复计划", "待执行恢复计划提示");

  return { backupFilename };
}

async function phaseVerifyAfterRestore(page) {
  log("phase verify-after-restore");
  assert.ok(config.backupFilename, "BROWSER_BACKUP_FILENAME is required for verify-after-restore");

  await gotoRoot(page);
  await waitForText(page, "登录控制台", "恢复后登录页");
  await fillInputByLabel(page, "邮箱", config.adminEmail);
  await fillInputByLabel(page, "密码", config.adminPassword);
  await page.locator(".login-form").getByRole("button", { name: "登录", exact: true }).click();

  await waitForAuthenticatedNav(page);
  await openSettingsPage(page);

  await clickSettingsNav(page, "审计日志");
  await waitForText(page, "审计日志", "审计日志分区");
  await clickButton(page, "刷新日志");
  await waitForText(page, "数据库恢复已完成", "审计日志恢复完成文案");
  await waitForText(page, config.backupFilename, "审计日志中的恢复备份文件名");

  await clickSettingsNav(page, "维护工具");
  await waitForText(page, "最近一次恢复结果", "维护工具最近恢复结果卡片");
  await waitForText(page, config.backupFilename, "维护工具中的恢复备份文件名");
  await waitForText(page, "MySQL / MariaDB 备份", "维护工具数据库类型文案");

  return { backupFilename: config.backupFilename };
}

async function runPhase(page) {
  switch (phase) {
    case "bootstrap-mysql":
      return phaseBootstrapMysql(page);
    case "install-and-restore-plan":
      return phaseInstallAndRestorePlan(page);
    case "verify-after-restore":
      return phaseVerifyAfterRestore(page);
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
  await page.screenshot({ path: screenshotPath, fullPage: true }).catch(() => {});
  console.error(error.stack || String(error));
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
