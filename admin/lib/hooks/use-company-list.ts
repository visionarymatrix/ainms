"use client";

import { useState, useEffect, useCallback } from "react";
import { listCompanies } from "@/lib/api/companies";
import type { Company } from "@/lib/api/companies";
import { isSuperAdmin } from "@/lib/auth/session";

export function useCompanyList() {
  const [companies, setCompanies] = useState<Company[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchCompanies = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await listCompanies();
      setCompanies(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch companies");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchCompanies();
  }, [fetchCompanies]);

  return { companies, loading, error, refetch: fetchCompanies };
}