#define GL_GLEXT_PROTOTYPES
#include <GL/glut.h>
#include <GL/glxew.h>
#include <stdio.h>
#include <iostream>

#include "texture_share_vk/opengl/texture_share_gl_client.h"

int main(int argc, char** argv)
{
	//create GL context
	glutInit(&argc, argv);
	glutInitDisplayMode(GLUT_RGBA);
	glutInitWindowSize(800, 600);
	glutCreateWindow("windowname");

	//create test checker image
	unsigned char texDat[64];
	for (int i = 0; i < 64; ++i)
		texDat[i] = ((i + (i / 8)) % 2) * 128 + 127;

	GLenum code;

	const std::string img_1_name = "test_gl";

	TextureShareGlClient client;
	client.InitImage(img_1_name, 8,8, GL_RGBA);

	u_char dat[4] = {255,255,0,255};
	client.ClearImage(img_1_name, dat);

	//upload to GPU texture
	GLuint tex_1;
	glGenTextures(1, &tex_1);
	glBindTexture(GL_TEXTURE_2D, tex_1);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, 8, 8, 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);

	dat[0] = 0;
	glClearTexImage(tex_1, 0, GL_RGBA, GL_UNSIGNED_BYTE, dat);
	client.SendImageBlit(img_1_name, tex_1, GL_TEXTURE_2D, {{0,0},{8,8}});
	std::cout << (code = glGetError()) << std::endl;

	glFlush();

	//upload to GPU texture
	GLuint tex_2;
	glGenTextures(1, &tex_2);
	glBindTexture(GL_TEXTURE_2D, tex_2);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, 8, 8, 0, GL_RGBA, GL_UNSIGNED_BYTE, nullptr);

	dat[0] = 255;
	dat[1] = 0;
	glClearTexImage(tex_2, 0, GL_RGBA, GL_UNSIGNED_BYTE, dat);
	client.RecvImageBlit(img_1_name, tex_2, GL_TEXTURE_2D, {{0,0}, {8,8}});
	std::cout << (code = glGetError()) << std::endl;

	glFlush();

	glBindTexture(GL_TEXTURE_2D, 0);

	//match projection to window resolution (could be in reshape callback)
	glMatrixMode(GL_PROJECTION);
	glOrtho(0, 800, 0, 600, -1, 1);
	glMatrixMode(GL_MODELVIEW);

	//clear and draw quad with texture (could be in display callback)
	glClear(GL_COLOR_BUFFER_BIT);
	glBindTexture(GL_TEXTURE_2D, tex_1);
	glEnable(GL_TEXTURE_2D);
	glBegin(GL_QUADS);
	glTexCoord2i(0, 0); glVertex2i(100, 100);
	glTexCoord2i(0, 1); glVertex2i(100, 500);
	glTexCoord2i(1, 1); glVertex2i(500, 500);
	glTexCoord2i(1, 0); glVertex2i(500, 100);
	glEnd();
	glDisable(GL_TEXTURE_2D);
	//glBindTexture(GL_TEXTURE_2D, 0);
	glFlush(); //don't need this with GLUT_DOUBLE and glutSwapBuffers

	//getchar(); //pause so you can see what just happened
	//System("pause"); //I think this works on windows

	return 0;
}
