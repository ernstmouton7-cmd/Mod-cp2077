export type ViolationKind =
  | "red_light"
  | "right_on_red_no_stop"
  | "stop_sign"
  | "speeding_minor"
  | "speeding_major"
  | "speeding_extreme"
  | "wrong_way"
  | "sidewalk"
  | "collision"
  | "hit_and_run"
  | "no_signal_lane"
  | "no_signal_turn"
  | "illegal_turn"
  | "no_headlights"
  | "seatbelt"
  | "fail_yield_emergency"
  | "tailgating"
  | "reckless"
  | "blocking_box"
  | "horn_abuse"
  | "evading";

export interface ViolationDef {
  label: string;
  /** Fine in eurodollars. */
  fine: number;
  /** License points. */
  points: number;
  /** Heat added when police see it. */
  heat: number;
  /** Seconds before the same violation can be charged again. */
  cooldown: number;
}

export const VIOLATIONS: Record<ViolationKind, ViolationDef> = {
  red_light: { label: "Ran a red light", fine: 400, points: 3, heat: 1, cooldown: 4 },
  right_on_red_no_stop: { label: "Right on red without stopping", fine: 150, points: 1, heat: 1, cooldown: 4 },
  stop_sign: { label: "Failed to stop at stop sign", fine: 200, points: 2, heat: 1, cooldown: 4 },
  speeding_minor: { label: "Speeding (10–25% over)", fine: 120, points: 1, heat: 1, cooldown: 25 },
  speeding_major: { label: "Speeding (25–50% over)", fine: 350, points: 3, heat: 1, cooldown: 25 },
  speeding_extreme: { label: "Excessive speed (50%+ over)", fine: 900, points: 6, heat: 2, cooldown: 25 },
  wrong_way: { label: "Driving the wrong way", fine: 500, points: 4, heat: 2, cooldown: 20 },
  sidewalk: { label: "Driving on the sidewalk", fine: 250, points: 2, heat: 1, cooldown: 15 },
  collision: { label: "Careless driving (collision)", fine: 300, points: 3, heat: 1, cooldown: 3 },
  hit_and_run: { label: "Leaving the scene of a collision", fine: 1500, points: 6, heat: 3, cooldown: 30 },
  no_signal_lane: { label: "Lane change without signalling", fine: 60, points: 0, heat: 1, cooldown: 15 },
  no_signal_turn: { label: "Turn without signalling", fine: 80, points: 1, heat: 1, cooldown: 10 },
  illegal_turn: { label: "Illegal turn", fine: 200, points: 2, heat: 1, cooldown: 8 },
  no_headlights: { label: "No headlights after dark", fine: 90, points: 1, heat: 1, cooldown: 60 },
  seatbelt: { label: "Not wearing a seatbelt", fine: 70, points: 0, heat: 1, cooldown: 60 },
  fail_yield_emergency: { label: "Failure to yield to emergency vehicle", fine: 450, points: 3, heat: 2, cooldown: 30 },
  tailgating: { label: "Following too closely", fine: 120, points: 2, heat: 1, cooldown: 20 },
  reckless: { label: "Reckless driving", fine: 800, points: 5, heat: 2, cooldown: 20 },
  blocking_box: { label: "Blocking the intersection", fine: 150, points: 1, heat: 1, cooldown: 20 },
  horn_abuse: { label: "Unnecessary use of horn", fine: 50, points: 0, heat: 1, cooldown: 30 },
  evading: { label: "Evading police", fine: 2500, points: 8, heat: 3, cooldown: 60 },
};

export type Witness = "police" | "camera" | "none";

const SPEED_TIERS: ViolationKind[] = ["speeding_minor", "speeding_major", "speeding_extreme"];

export interface ViolationRecord {
  kind: ViolationKind;
  label: string;
  time: number;
  fine: number;
  points: number;
  witness: Witness;
  /** Fines become due when the ticket is written (police) or immediately (camera). */
  paid: boolean;
  x: number;
  y: number;
}

/** The player's driving record and wallet. */
export class DrivingRecord {
  money = 5000;
  points = 0;
  readonly maxPoints = 12;
  readonly history: ViolationRecord[] = [];
  /** Witnessed-by-police violations awaiting a ticket. */
  readonly pending: ViolationRecord[] = [];
  heat = 0;
  /** Distance (m) driven without any violation. */
  cleanDistance = 0;
  ticketsIssued = 0;
  totalFines = 0;
  private lastCharged: Map<ViolationKind, number> = new Map();

  suspended(): boolean {
    return this.points >= this.maxPoints;
  }

  canCharge(kind: ViolationKind, now: number): boolean {
    const tier = SPEED_TIERS.indexOf(kind);
    if (tier >= 0) {
      // Speeding tiers share one cooldown; a higher tier may upgrade after 8 s.
      for (let i = 0; i < SPEED_TIERS.length; i++) {
        const t = this.lastCharged.get(SPEED_TIERS[i]);
        if (t === undefined) continue;
        const age = now - t;
        if (i >= tier && age < VIOLATIONS[kind].cooldown) return false;
        if (i < tier && age < 8) return false;
      }
      return true;
    }
    const last = this.lastCharged.get(kind);
    return last === undefined || now - last >= VIOLATIONS[kind].cooldown;
  }

  add(kind: ViolationKind, now: number, witness: Witness, x: number, y: number): ViolationRecord {
    const d = VIOLATIONS[kind];
    this.lastCharged.set(kind, now);
    const rec: ViolationRecord = { kind, label: d.label, time: now, fine: d.fine, points: d.points, witness, paid: false, x, y };
    this.history.push(rec);
    if (this.history.length > 200) this.history.shift();
    this.cleanDistance = 0;
    if (witness === "camera") this.charge([rec]);
    else if (witness === "police") this.pending.push(rec);
    return rec;
  }

  /** Charge records (pay fines, add points). Returns total fine. */
  charge(recs: ViolationRecord[]): number {
    let total = 0;
    for (const r of recs) {
      if (r.paid) continue;
      r.paid = true;
      total += r.fine;
      this.points += r.points;
    }
    this.money -= total;
    this.totalFines += total;
    return total;
  }

  /** Ticket for all pending police-witnessed violations. */
  issueTicket(): { total: number; points: number; count: number } {
    const recs = this.pending.splice(0, this.pending.length);
    const pts = recs.reduce((a, r) => a + r.points, 0);
    const total = this.charge(recs);
    this.ticketsIssued++;
    return { total, points: pts, count: recs.length };
  }

  /** Points expire slowly with clean driving (1 point per 2 km). */
  driveClean(meters: number): void {
    this.cleanDistance += meters;
    if (this.cleanDistance > 2000 && this.points > 0) {
      this.points--;
      this.cleanDistance = 0;
    }
  }
}
