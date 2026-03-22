const { test, expect } = require("@playwright/test");
const { open3D, wheelOn } = require("./helpers");

function faceSelector(face) {
  return `.three-d-face[data-layer="${face.layer}"][data-row="${face.row}"][data-col="${face.col}"][data-direction="${face.direction}"]`;
}

async function interactiveFace(page, kind) {
  return page.evaluate((requestedKind) => {
    const faces = [...document.querySelectorAll(".three-d-face.is-interactive")];
    for (const face of faces) {
      const layer = Number.parseInt(face.dataset.layer, 10);
      const row = Number.parseInt(face.dataset.row, 10);
      const col = Number.parseInt(face.dataset.col, 10);
      const cell = state.cells[layer]?.[row]?.[col];
      if (!cell) {
        continue;
      }

      const matches =
        requestedKind === "revealed" ? cell.revealed === true : cell.revealed !== true;
      if (!matches) {
        continue;
      }

      return {
        layer,
        row,
        col,
        direction: face.dataset.direction,
      };
    }

    return null;
  }, kind);
}

async function interactFace(page, face) {
  await page.evaluate((targetFace) => {
    const selector = `.three-d-face[data-layer="${targetFace.layer}"][data-row="${targetFace.row}"][data-col="${targetFace.col}"][data-direction="${targetFace.direction}"]`;
    const faceEl = document.querySelector(selector);
    if (!faceEl) {
      throw new Error(`Could not find face for ${selector}`);
    }
    interactWithFace(faceEl);
  }, face);
}

async function clickFaceNote(page, face, slot = "top") {
  await page.evaluate(({ targetFace, noteSlot }) => {
    const selector = `.three-d-face[data-layer="${targetFace.layer}"][data-row="${targetFace.row}"][data-col="${targetFace.col}"][data-direction="${targetFace.direction}"]`;
    const button = document.querySelector(
      `${selector} ${noteSlot === "top" ? ".cell-note" : ".cell-note-secondary"}`,
    );
    if (!(button instanceof HTMLElement)) {
      throw new Error(`Could not find ${noteSlot} note button for ${selector}`);
    }
    button.click();
  }, { targetFace: face, noteSlot: slot });
}

test("3D new puzzle modal reflects current dims and seeded share reuses the same stored id", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: "http://127.0.0.1:3000",
  });
  await open3D(page);

  await page.click("#new-random-3d");
  await expect(page.locator("#new-puzzle-3d-modal")).toBeVisible();
  await expect(page.locator("#new-puzzle-3d-depth-value")).toHaveText("2");
  await expect(page.locator("#new-puzzle-3d-rows-value")).toHaveText("2");
  await expect(page.locator("#new-puzzle-3d-cols-value")).toHaveText("2");
  await page.click("#new-puzzle-3d-cancel");

  await page.click("#share-3d");
  await expect
    .poll(() => page.evaluate(() => state.sharedStoredPuzzleId))
    .toMatch(/^[0-9a-f]{6}$/);
  const firstId = await page.evaluate(() => state.sharedStoredPuzzleId);
  await page.click("#share-3d");
  const secondId = await page.evaluate(() => state.sharedStoredPuzzleId);
  expect(secondId).toBe(firstId);
});

test("3D revealed faces toggle clue visibility and unrevealed faces open the guess flow without trapping the modal", async ({
  page,
}) => {
  await open3D(page);
  await wheelOn(page, "#board-3d-shell", 120, -80, false);

  const revealed = await interactiveFace(page, "revealed");
  expect(revealed).not.toBeNull();
  await interactFace(page, revealed);
  await expect
    .poll(() => page.evaluate(() => state.hiddenClues.size))
    .toBe(1);

  const hidden = await interactiveFace(page, "hidden");
  expect(hidden).not.toBeNull();
  await interactFace(page, hidden);
  await expect(page.locator("#guess-modal")).toBeVisible();
  const hiddenAnswer = await page.evaluate(
    (face) => state.cells[face.layer][face.row][face.col].answer,
    hidden,
  );
  await page.click(hiddenAnswer === "innocent" ? "#guess-criminal" : "#guess-innocent");
  await expect(page.locator("#error-title")).toHaveText("⚠️ Not enough evidence!");
  await expect(page.locator("#guess-modal")).toBeHidden();
  await page.click("#error-dismiss");
  await expect(page.locator("#error-modal")).toBeHidden();
});

test("3D corner notes are shared across faces and reset view restores default rotation and zoom", async ({
  page,
}) => {
  await open3D(page);

  const hidden = await interactiveFace(page, "hidden");
  expect(hidden).not.toBeNull();
  await clickFaceNote(page, hidden, "top");
  await expect
    .poll(() =>
      page.evaluate((targetFace) => {
        const selector = `.three-d-face[data-layer="${targetFace.layer}"][data-row="${targetFace.row}"][data-col="${targetFace.col}"] .cell-note`;
        const notes = [...document.querySelectorAll(selector)];
        return notes.length > 0 && notes.every((note) => note.classList.contains("note-yellow"));
      }, hidden),
    )
    .toBe(true);

  await wheelOn(page, "#board-3d-shell", 160, -110, false);
  await wheelOn(page, "#board-3d-shell", 0, -320, true);
  const changed = await page.evaluate(() => ({
    rotationX: state.rotationX,
    rotationY: state.rotationY,
    zoom: state.userZoomScale,
  }));
  expect(changed.rotationX).not.toBe(-22);
  expect(changed.rotationY).not.toBe(35);
  expect(changed.zoom).not.toBe(1);

  await page.click("#reset-view-3d");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          Math.abs(state.rotationX - -22) < 0.25 &&
          Math.abs(state.rotationY - 35) < 0.25 &&
          Math.abs(state.userZoomScale - 1) < 0.02,
      ),
    )
    .toBe(true);
});

test("3D progress and view state persist across reload", async ({ page }) => {
  await open3D(page);

  const hidden = await interactiveFace(page, "hidden");
  const revealed = await interactiveFace(page, "revealed");
  expect(hidden).not.toBeNull();
  expect(revealed).not.toBeNull();

  await clickFaceNote(page, hidden, "top");
  await interactFace(page, revealed);
  await wheelOn(page, "#board-3d-shell", 120, -90, false);
  await wheelOn(page, "#board-3d-shell", 0, -220, true);

  const beforeReload = await page.evaluate(() => ({
    rotationX: state.rotationX,
    rotationY: state.rotationY,
    zoom: state.userZoomScale,
    hiddenClues: [...state.hiddenClues],
  }));

  await page.reload();
  await expect(page.locator("#board-3d-scene .three-d-face")).toHaveCount(48);
  await expect(page.locator(`${faceSelector(hidden)} .cell-note`)).toHaveClass(/note-yellow/);
  await expect
    .poll(() =>
      page.evaluate(() => ({
        rotationX: state.rotationX,
        rotationY: state.rotationY,
        zoom: state.userZoomScale,
        hiddenClues: [...state.hiddenClues],
      })),
    )
    .toEqual(beforeReload);
});

test("3D first load shows Start, reload with progress shows Resume, and completed puzzles skip the gate", async ({
  page,
}) => {
  await open3D(page, "/3d/?seed=921d06880b8b&depth=2&rows=2&cols=2", { start: false });
  await expect(page.locator("#start-button")).toHaveText("Start");
  await page.waitForTimeout(250);
  expect(await page.evaluate(() => currentTimerElapsedMs())).toBe(0);

  await page.click("#start-button");
  await page.waitForTimeout(220);
  const startedElapsed = await page.evaluate(() => currentTimerElapsedMs());
  expect(startedElapsed).toBeGreaterThan(0);

  await page.reload();
  await expect(page.locator("#start-button")).toHaveText("Resume");
  const beforeResume = await page.evaluate(() => currentTimerElapsedMs());
  await page.waitForTimeout(250);
  const stillPaused = await page.evaluate(() => currentTimerElapsedMs());
  expect(stillPaused - beforeResume).toBeLessThan(80);

  await page.click("#start-button");
  await page.waitForTimeout(220);
  const afterResume = await page.evaluate(() => currentTimerElapsedMs());
  expect(afterResume - stillPaused).toBeGreaterThan(120);

  await page.evaluate(async () => {
    while (!allTilesMarked()) {
      let solvedNext = false;

      for (let layer = 0; layer < state.cells.length && !solvedNext; layer += 1) {
        for (let row = 0; row < state.cells[layer].length && !solvedNext; row += 1) {
          for (let col = 0; col < state.cells[layer][row].length && !solvedNext; col += 1) {
            if (state.cells[layer][row][col].revealed) {
              continue;
            }

            for (const guess of ["innocent", "criminal"]) {
              state.modalCell = { layer, row, col };
              try {
                await revealGuess(guess);
                solvedNext = true;
                break;
              } catch {}
            }
          }
        }
      }

      if (!solvedNext) {
        throw new Error("Could not find a forced move while solving the 3D puzzle.");
      }
    }
  });

  await page.reload();
  await expect(page.locator("#start-modal")).toBeHidden();
  await expect(page.locator("#finish-modal")).toBeHidden();
  await expect(page.locator("#share-3d")).toHaveText("🏆 Share Results");
});

test("3D completion switches share to trophy mode and reopens the results modal", async ({
  page,
}) => {
  await open3D(page);

  await page.evaluate(() => {
    state.cells.forEach((layer) => {
      layer.forEach((row) => {
        row.forEach((cell) => {
          cell.revealed = true;
          cell.revealed_answer = cell.revealed_answer ?? "innocent";
        });
      });
    });
    state.timerElapsedMs = 0;
    state.timerStartedAt = Date.now() - 1600;
    state.timerCompletedAt = null;
    completePuzzleIfNeeded();
    renderBoard();
  });

  await expect(page.locator("#finish-modal")).toBeVisible();
  await expect(page.locator("#share-3d")).toHaveText("🏆 Share Results");
  await page.click("#finish-dismiss");
  await expect(page.locator("#finish-modal")).toBeHidden();
  await page.click("#share-3d");
  await expect(page.locator("#finish-modal")).toBeVisible();
});
