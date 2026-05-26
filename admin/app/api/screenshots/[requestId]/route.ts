import { NextRequest, NextResponse } from 'next/server';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://173.249.47.143:8440";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ requestId: string }> }
) {
  const { requestId } = await params;
  
  const authHeader = request.headers.get('authorization');
  const tokenFromHeader = authHeader?.replace('Bearer ', '');
  const tokenFromQuery = request.nextUrl.searchParams.get('token');
  const token = tokenFromHeader || tokenFromQuery;
  
  if (!token) {
    return NextResponse.json({ error: 'unauthorized' }, { status: 401 });
  }

  try {
    const backendUrl = `${API_BASE_URL}/v1/screenshots/${requestId}/image`;
    const resp = await fetch(backendUrl, {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!resp.ok) {
      return NextResponse.json({ error: 'not found' }, { status: resp.status });
    }

    const imageData = await resp.arrayBuffer();
    const contentType = resp.headers.get('content-type') || 'image/png';
    
    return new NextResponse(imageData, {
      headers: {
        'Content-Type': contentType,
        'Cache-Control': 'public, max-age=86400',
      },
    });
  } catch (error) {
    console.error('Error proxying screenshot:', error);
    return NextResponse.json({ error: 'internal server error' }, { status: 500 });
  }
}
