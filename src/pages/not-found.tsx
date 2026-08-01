import { Link } from "wouter";
import { AppLayout } from "@/components/layout";
import { Terminal, ArrowLeft } from "lucide-react";

export default function NotFound() {
  return (
    <AppLayout>
      <div className="flex flex-col items-center justify-center h-full gap-5 text-center px-6 animate-fade-in">
        <div className="h-14 w-14 rounded-2xl bg-black/8 border border-black/15 flex items-center justify-center">
          <Terminal className="h-7 w-7 text-black/60" />
        </div>
        <div className="space-y-1.5">
          <h1 className="text-4xl font-bold font-mono text-black/80">404</h1>
          <p className="text-sm font-medium text-muted-foreground">الصفحة غير موجودة</p>
          <p className="text-xs text-muted-foreground/50">هذا المسار غير موجود في بوب كورن ستوديو.</p>
        </div>
        <Link
          href="/"
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl border border-black/25 text-black text-sm hover:bg-black/8 transition-all"
        >
          <ArrowLeft className="h-4 w-4" />
          العودة للوكيل
        </Link>
      </div>
    </AppLayout>
  );
}
