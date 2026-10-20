import { useEffect, useState, type RefObject } from "react";

/** Delivery requires a rendered element intersecting the visible viewport.
 * This still cannot establish that the person noticed it. */
export function useVisible(ref: RefObject<HTMLElement | null>, enabled: boolean) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const element = ref.current;
    if (!enabled || !element) { setVisible(false); return; }
    let intersects = false;
    const update = () => {
      const rect = element.getBoundingClientRect();
      setVisible(intersects && document.visibilityState === "visible" && !document.querySelector('[aria-modal="true"]') && rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < window.innerHeight);
    };
    const observer = typeof IntersectionObserver === "undefined" ? null : new IntersectionObserver(([entry]) => { intersects = entry.isIntersecting; update(); }, { threshold: 0.05 });
    observer?.observe(element);
    const modalObserver = new MutationObserver(update);
    modalObserver.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["aria-modal", "hidden", "open"] });
    document.addEventListener("visibilitychange", update);
    window.addEventListener("focus", update);
    return () => { observer?.disconnect(); modalObserver.disconnect(); document.removeEventListener("visibilitychange", update); window.removeEventListener("focus", update); };
  }, [ref, enabled]);
  return visible;
}
