/**
 * Formats integer paisa into Indian currency format: ₹X,XX,XXX.XX
 */
export function formatINR(paisa: number): string {
  const isNegative = paisa < 0;
  const absPaisa = Math.abs(paisa);
  const rupees = Math.floor(absPaisa / 100);
  const remainingPaisa = absPaisa % 100;

  const paisaStr = remainingPaisa < 10 ? `0${remainingPaisa}` : `${remainingPaisa}`;

  // Indian number grouping: last 3 digits, then groups of 2
  const rupeeStr = rupees.toString();
  let formattedRupees = '';

  if (rupeeStr.length <= 3) {
    formattedRupees = rupeeStr;
  } else {
    const lastThree = rupeeStr.slice(-3);
    const otherDigits = rupeeStr.slice(0, -3);
    const formattedOthers = otherDigits.replace(/\B(?=(\d{2})+(?!\d))/g, ',');
    formattedRupees = `${formattedOthers},${lastThree}`;
  }

  const prefix = isNegative ? '-₹' : '₹';
  return `${prefix}${formattedRupees}.${paisaStr}`;
}

export function formatBps(bps: number): string {
  const pct = (bps / 100).toFixed(2);
  return `${pct}%`;
}

export function formatDate(isoStr: string): string {
  try {
    const d = new Date(isoStr);
    return d.toLocaleDateString('en-IN', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  } catch {
    return isoStr;
  }
}
