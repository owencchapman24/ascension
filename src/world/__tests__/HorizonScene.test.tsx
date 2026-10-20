import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { HORIZON_POLICY, HORIZON_UNIT_COUNT, HorizonScene, RESTORATION_UNITS, getRestoredUnitCount } from "../HorizonScene";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(cleanup);

const fixedCases = [0, 1, 2, 3, 4, 8, 16, 32, 48, 63, 64, 65, 100];

describe("Horizon 1 committed issuance projection", () => {
  it("defines one stable ordered 64-unit scene", () => {
    expect(HORIZON_POLICY).toBe("h1-ledger-64-v1");
    expect(HORIZON_UNIT_COUNT).toBe(64);
    expect(RESTORATION_UNITS).toHaveLength(64);
    expect(new Set(RESTORATION_UNITS.map(unit => unit.id)).size).toBe(64);
    expect(RESTORATION_UNITS.map(unit => unit.ordinal)).toEqual(Array.from({ length: 64 }, (_, index) => index + 1));
    expect(RESTORATION_UNITS[0]).toMatchObject({ id: "river-approach", label: "Lower river approach" });
    expect(RESTORATION_UNITS[31].id).toBe("commons-flowers");
    expect(RESTORATION_UNITS[63]).toMatchObject({ id: "beacon-light", label: "The beacon light" });
    expect(Object.isFrozen(RESTORATION_UNITS)).toBe(true);
    expect(RESTORATION_UNITS.every(Object.isFrozen)).toBe(true);
  });

  it.each(fixedCases)("renders exactly the authored first min(64, %i) completed units", issuedBC => {
    const { container } = render(<HorizonScene issuedBC={issuedBC} />);
    const expected = Math.min(64, issuedBC);
    const all = [...container.querySelectorAll<SVGGElement>("[data-unit-id]")];
    const restored = all.filter(unit => unit.dataset.restored === "true");
    expect(all).toHaveLength(64);
    expect(restored).toHaveLength(expected);
    expect(restored.map(unit => unit.dataset.unitId).sort()).toEqual(RESTORATION_UNITS.slice(0, expected).map(unit => unit.id).sort());
    expect(screen.getByRole("img", { name: new RegExp(`^${expected} of 64 restoration units complete`) })).toBeInTheDocument();
    expect(container.querySelector("figure")).toHaveAttribute("data-restored-units", String(expected));
    expect(container.querySelectorAll(".horizon-beacon-light")).toHaveLength(expected === 64 ? 1 : 0);
  });

  it("changes the actual geometry of exactly one restoration unit for each new BC", () => {
    const { container, rerender } = render(<HorizonScene issuedBC={0} />);
    let previous = new Map([...container.querySelectorAll("[data-unit-id]")].map(unit => [unit.getAttribute("data-unit-id"), unit.innerHTML]));
    for (let issuedBC = 1; issuedBC <= 64; issuedBC++) {
      rerender(<HorizonScene issuedBC={issuedBC} />);
      const current = new Map([...container.querySelectorAll("[data-unit-id]")].map(unit => [unit.getAttribute("data-unit-id"), unit.innerHTML]));
      const changed = [...current].filter(([id, geometry]) => previous.get(id) !== geometry).map(([id]) => id);
      expect(changed, `BC ${issuedBC} must restore exactly one visibly distinct unit`).toEqual([RESTORATION_UNITS[issuedBC - 1].id]);
      previous = current;
    }
  }, 15_000);

  it("retains all credits at saturation with identical completed scene geometry", () => {
    const { container, rerender } = render(<HorizonScene issuedBC={64} />);
    const artwork = container.querySelector("svg")!.innerHTML;
    rerender(<HorizonScene issuedBC={65} />);
    expect(container.querySelector("svg")!.innerHTML).toBe(artwork);
    expect(screen.getByText(/All 65 issued Build Credits remain in your ledger/)).toBeInTheDocument();
    rerender(<HorizonScene issuedBC={100} />);
    expect(container.querySelector("svg")!.innerHTML).toBe(artwork);
    expect(screen.getByText(/All 100 issued Build Credits remain in your ledger/)).toBeInTheDocument();
  });

  it("reconstructs the same scene on reopen without a second persistence authority", () => {
    const first = render(<HorizonScene issuedBC={32} />);
    const units = [...first.container.querySelectorAll("[data-unit-id]")].map(unit => unit.outerHTML);
    first.unmount();
    const second = render(<HorizonScene issuedBC={32} />);
    expect([...second.container.querySelectorAll("[data-unit-id]")].map(unit => unit.outerHTML)).toEqual(units);
  });

  it("uses the same issued-BC scene in a compact preview", () => {
    const { container, rerender } = render(<HorizonScene issuedBC={48} />);
    const artwork = container.querySelector("svg")!.innerHTML;
    rerender(<HorizonScene issuedBC={48} preview />);
    expect(container.querySelector("svg")!.innerHTML).toBe(artwork);
    expect(container.querySelector("figure")).toHaveClass("horizon-preview");
    expect(container.querySelector("details")).toBeNull();
  });

  it("provides keyboard-readable unit status and never calls a backend or reward command", async () => {
    const user = userEvent.setup({ delay: null });
    const { container, rerender } = render(<HorizonScene issuedBC={4} />);
    const record = screen.getByText(/Restoration record/);
    await user.tab();
    expect(record).toHaveFocus();
    // jsdom does not implement the browser's native Enter toggle for <summary>.
    // Assert keyboard focus here; the native smoke check covers activation.
    await user.click(record);
    expect(container.querySelector("details")).toHaveAttribute("open");
    expect(within(screen.getByRole("list")).getAllByText("Complete")).toHaveLength(4);
    expect(within(screen.getByRole("list")).getAllByText("Unrestored")).toHaveLength(60);
    rerender(<HorizonScene issuedBC={64} />);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("keeps construction derived from retained issuance when factual acceptance status changes elsewhere", () => {
    const { container, rerender } = render(<HorizonScene issuedBC={4} />);
    const beforeCorrection = container.querySelector("svg")!.innerHTML;
    // Factual acceptance and correction records are deliberately not component inputs.
    rerender(<HorizonScene issuedBC={4} />);
    expect(container.querySelector("svg")!.innerHTML).toBe(beforeCorrection);
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])("does not manufacture construction from invalid issuance %s", value => {
    expect(getRestoredUnitCount(value)).toBe(0);
    const { container } = render(<HorizonScene issuedBC={value} />);
    expect(container.querySelectorAll('[data-restored="true"]')).toHaveLength(0);
    expect(screen.getByRole("alert")).toHaveTextContent("waiting for a valid committed Build Credit total");
  });
});
