import { useEffect, useState } from "react";

/** 2.117.0: TIQR in a phone's browser - narrower than Tailwind's `md`, where
 * the rail gives way to the tab bar at the bottom (Layout's PhoneNav). The
 * desktop app's window never gets this narrow (1080px minimum), so nothing
 * there changes. */
const PHONE_QUERY = "(max-width: 767px)";

/** Reactive, like useNarrowTables: re-renders only when the width crosses the
 * line (a phone turned sideways), not on every pixel. */
export function usePhone(): boolean {
  const [phone, setPhone] = useState(() => typeof window !== "undefined" && window.matchMedia(PHONE_QUERY).matches);

  useEffect(() => {
    const mq = window.matchMedia(PHONE_QUERY);
    const onChange = () => setPhone(mq.matches);
    onChange();
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  return phone;
}
