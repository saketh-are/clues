const { test, expect } = require("@playwright/test");

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function buildAndShareTinyPuzzle(page) {
  await page.goto("/edit");
  await expect(page.getByRole("heading", { name: "Clues Editor" })).toBeVisible();
  await expect(page.locator("#rows-select")).toBeEnabled();

  await page.selectOption("#rows-select", "2");
  await page.selectOption("#cols-select", "2");
  await page.fill("#author-input", "E2E Bot");
  await page.click("#reset-editor");

  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(4);

  await page.locator("#editor-board .editor-cell").first().click();
  await expect(page.locator("#set-initial-innocent")).toBeVisible();
  await page.click("#set-initial-innocent");

  await expect(page.locator("#generate-remaining")).toBeEnabled();
  await page.locator("#generate-remaining").click({ modifiers: ["Shift"] });

  const shareButton = page.locator("#share-editor");
  await expect(shareButton).toBeVisible();
  await expect(shareButton).toBeEnabled();
  await page.click("#share-editor");

  const sharedLink = page.locator("#editor-share-link a");
  await expect(sharedLink).toBeVisible();
  const href = await sharedLink.getAttribute("href");
  const storedPuzzleId = new URL(href, "http://127.0.0.1:3000").pathname.split("/").pop();
  expect(storedPuzzleId).toMatch(/^[0-9a-f]{6}$/);
  return storedPuzzleId;
}

async function startPuzzleIfNeeded(page) {
  const startButton = page.locator("#start-button");
  if (await startButton.isVisible()) {
    await startButton.click();
  }
}

async function solveStoredPuzzleInProgressionOrder(page, request, storedPuzzleId) {
  const response = await request.post("/api/editor/open", {
    data: {
      source: {
        kind: "stored",
        stored_puzzle_id: storedPuzzleId,
      },
    },
  });
  expect(response.ok()).toBeTruthy();
  const opened = await response.json();
  const initialReveal = opened.draft.initial_reveal;
  const addClueSteps = opened.progression.filter(
    (step) =>
      step.kind === "add_clue" &&
      !(initialReveal && step.row === initialReveal.row && step.col === initialReveal.col),
  );
  expect(addClueSteps.length).toBeGreaterThan(0);

  await page.goto(`/p/${storedPuzzleId}`);
  await expect(page.locator("#puzzle-title")).toHaveText("Clues by E2E Bot");
  await startPuzzleIfNeeded(page);

  for (const step of addClueSteps) {
    const authoredCell = opened.draft.cells[step.row][step.col];
    const card = page
      .locator("#board .cell")
      .filter({
        has: page.locator(".cell-name", {
          hasText: new RegExp(`^${escapeRegex(authoredCell.name)}$`),
        }),
      })
      .first();

    await expect(card).toBeVisible();
    await card.click();
    await expect(page.locator("#guess-modal")).toBeVisible();

    await page.click(
      authoredCell.answer === "innocent" ? "#guess-innocent" : "#guess-criminal",
    );
    await expect(page.locator("#guess-modal")).toBeHidden();
  }
}

test("2D play shell opens the new puzzle modal", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Clues" })).toBeVisible();

  await page.click("#new-random");
  await expect(page.locator("#new-puzzle-modal")).toBeVisible();
  await expect(page.locator("#new-puzzle-rows-value")).toContainText(/\d+/);
  await expect(page.locator("#new-puzzle-cols-value")).toContainText(/\d+/);
});

test("editor can share a 2D puzzle and the stored puzzle can be solved in play UI", async ({
  page,
  request,
  context,
}) => {
  test.slow();
  await context.grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: "http://127.0.0.1:3000",
  });

  const storedPuzzleId = await buildAndShareTinyPuzzle(page);
  await solveStoredPuzzleInProgressionOrder(page, request, storedPuzzleId);

  await expect(page.locator("#finish-modal")).toBeVisible();
  await expect(page.locator("#finish-title")).toHaveText("Puzzle Complete");
  await expect(page.locator("#share-puzzle")).toHaveText("🏆 Share Results");
});
